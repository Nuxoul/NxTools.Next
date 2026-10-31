//! Floating assistant is only another presentation/input source, never a download engine.
use super::*;
use crate::tools::floating_assistant::application::DownloadProjection;

impl DownloadApplication {
    pub(crate) fn floating_assistant_projection(&self) -> DownloadProjection {
        let mut projection = DownloadProjection::default();
        let (mut downloaded, mut total) = (0_u64, Some(0_u64));
        for entry in self.tasks.iter() {
            let status = entry.task.status();
            if !matches!(
                status,
                DownloadStatus::Probing
                    | DownloadStatus::Waiting
                    | DownloadStatus::Downloading
                    | DownloadStatus::Finalizing
                    | DownloadStatus::Verifying
            ) {
                continue;
            }
            projection.active_count += 1;
            let progress = self.latest_progress.get(&entry.task.id());
            downloaded = downloaded.saturating_add(
                progress.map_or(entry.downloaded_bytes, |value| value.downloaded_bytes),
            );
            total = total
                .zip(
                    progress
                        .and_then(|value| value.total_bytes)
                        .or(entry.total_bytes),
                )
                .map(|(a, b)| a.saturating_add(b));
            if status == DownloadStatus::Downloading {
                projection.downloading += 1;
                projection.speed = projection
                    .speed
                    .saturating_add(progress.map_or(0, |value| value.bytes_per_second));
            }
        }
        // Unknown total is indeterminate, not zero (nor 100% when known tasks finish).
        projection.progress = if projection.active_count == 0 {
            None
        } else {
            total
                .filter(|total| *total > 0)
                .map(|total| (downloaded as f64 / total as f64 * 100.0).clamp(0.0, 100.0) as f32)
        };
        projection
    }

    pub(crate) fn open_floating_download(
        &mut self,
        url: Option<String>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        // Mirror the ordinary blank form's request defaults; only the URL differs.
        // The existing Add Download owner probes, retains the draft and requires confirmation.
        let source = url.map(|source_url| {
            new_download_view::NewDownloadInitialSource::Download(DownloadRequest {
                source_url,
                save_directory: self.default_directory.clone(),
                save_directory_explicit: false,
                file_name: None,
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                certificate_policy: Default::default(),
                connection_limit: Some(self.default_connection_limit()),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: Some(self.default_adaptive_segment_policy()),
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: Default::default(),
            })
        });
        self.open_new_download_window_with_request(source, None, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;

    fn fixture(cx: &mut TestAppContext, label: &str) -> (Entity<DownloadApplication>, PathBuf) {
        cx.update(gpui_kit::init);
        super::super::tests::download_application_for_window_test(cx, label)
    }

    fn enqueue(
        application: &Entity<DownloadApplication>,
        cx: &mut TestAppContext,
        directory: &Path,
        name: &str,
    ) -> DownloadEngineTask {
        let id = application.update(cx, |application, _| {
            application
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: format!("https://example.test/{name}"),
                        save_directory: directory.to_path_buf(),
                        save_directory_explicit: false,
                        file_name: Some(name.into()),
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
                        duplicate_strategy: Default::default(),
                    },
                    RequestOrigin::Internal,
                )
                .expect("enqueue fixture")
        });
        application.read_with(cx, |application, _| {
            application
                .engine
                .task(id)
                .expect("read fixture")
                .expect("task")
        })
    }

    fn advance(entry: &mut DownloadEngineTask, status: DownloadStatus) {
        let path: &[DownloadStatus] = match status {
            DownloadStatus::Probing => &[DownloadStatus::Probing],
            DownloadStatus::Waiting => &[DownloadStatus::Probing, DownloadStatus::Waiting],
            DownloadStatus::Downloading
            | DownloadStatus::Finalizing
            | DownloadStatus::Completed => &[
                DownloadStatus::Probing,
                DownloadStatus::Waiting,
                DownloadStatus::Downloading,
            ],
            _ => panic!("unsupported fixture status"),
        };
        for (index, state) in path.iter().copied().enumerate() {
            entry
                .task
                .transition_to(state, index as i64 + 2)
                .expect("fixture transition");
        }
        if status == DownloadStatus::Finalizing || status == DownloadStatus::Completed {
            entry
                .task
                .transition_to(DownloadStatus::Finalizing, 5)
                .expect("finalizing");
        }
        if status == DownloadStatus::Completed {
            entry
                .task
                .transition_to(DownloadStatus::Completed, 6)
                .expect("completed");
        }
    }

    fn progress(
        entry: &DownloadEngineTask,
        done: u64,
        total: Option<u64>,
        speed: u64,
    ) -> DownloadProgress {
        DownloadProgress {
            task_id: entry.task.id(),
            downloaded_bytes: done,
            total_bytes: total,
            bytes_per_second: speed,
            active_connections: 1,
            segments: Vec::new(),
        }
    }

    #[gpui_kit::test]
    fn floating_projection_uses_guarded_live_tasks_and_weighted_known_bytes(
        cx: &mut TestAppContext,
    ) {
        let (application, directory) = fixture(cx, "floating-projection");
        let mut first = enqueue(&application, cx, &directory, "first.bin");
        let mut second = enqueue(&application, cx, &directory, "second.bin");
        let mut waiting = enqueue(&application, cx, &directory, "waiting.bin");
        let mut paused = enqueue(&application, cx, &directory, "paused.bin");
        let mut finished = enqueue(&application, cx, &directory, "finished.bin");
        advance(&mut first, DownloadStatus::Downloading);
        advance(&mut second, DownloadStatus::Downloading);
        advance(&mut waiting, DownloadStatus::Waiting);
        advance(&mut finished, DownloadStatus::Completed);
        advance(&mut paused, DownloadStatus::Downloading);
        paused
            .task
            .transition_to(DownloadStatus::Paused, 7)
            .expect("paused task");
        application.update(cx, |app, _| {
            app.replace_tasks_for_test(vec![
                first.clone(),
                second.clone(),
                waiting.clone(),
                paused.clone(),
                finished.clone(),
            ]);
            app.latest_progress
                .insert(first.task.id(), progress(&first, 25, Some(100), 7));
            app.latest_progress
                .insert(second.task.id(), progress(&second, 100, Some(300), 11));
            app.latest_progress
                .insert(waiting.task.id(), progress(&waiting, 50, Some(100), 500));
            app.latest_progress
                .insert(finished.task.id(), progress(&finished, 300, Some(300), 900));
            app.latest_progress
                .insert(paused.task.id(), progress(&paused, 80, Some(100), 700));
            let projected = app.floating_assistant_projection();
            assert_eq!(
                (
                    projected.active_count,
                    projected.downloading,
                    projected.speed
                ),
                (3, 2, 18)
            );
            assert_eq!(projected.progress, Some(35.0)); // (25+100+50)/(100+300+100)
            app.latest_progress.remove(&first.task.id());
            assert_eq!(
                app.floating_assistant_projection().speed,
                11,
                "missing live progress cannot invent speed"
            );
            app.latest_progress
                .insert(first.task.id(), progress(&first, 25, Some(100), 7));
            app.latest_progress
                .insert(second.task.id(), progress(&second, 100, None, 11));
            // Remove the stored known size as well: unknown cannot be represented as 100%.
            Rc::make_mut(&mut app.tasks)[1].total_bytes = None;
            assert_eq!(app.floating_assistant_projection().progress, None);
            app.replace_tasks_for_test(vec![finished]);
            assert_eq!(
                app.floating_assistant_projection(),
                DownloadProjection::default()
            );
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn floating_add_download_reuses_blank_and_prefill_requires_normal_probe_and_confirmation(
        cx: &mut TestAppContext,
    ) {
        let (application, directory) = fixture(cx, "floating-add");
        application.update(cx, |app, cx| {
            app.open_floating_download(None, cx).expect("blank form")
        });
        let (first_window, view) = application.read_with(cx, |app, _| {
            (
                app.new_download_window.expect("normal window"),
                app.new_download_view.clone().expect("form"),
            )
        });
        assert_eq!(
            view.read_with(cx, |view, cx| view.url.read(cx).value().to_string()),
            ""
        );
        application.update(cx, |app, cx| {
            app.open_floating_download(None, cx).expect("reuse")
        });
        assert_eq!(
            application.read_with(cx, |app, _| app.new_download_window),
            Some(first_window)
        );
        let url = "http://127.0.0.1:9/floating.bin";
        let conflict = application.update(cx, |app, cx| {
            app.open_floating_download(Some(url.into()), cx)
        });
        assert!(
            conflict.is_err(),
            "must not overwrite an existing dialog draft"
        );
        assert_eq!(
            view.read_with(cx, |view, cx| view.url.read(cx).value().to_string()),
            ""
        );
        // Close the old window through the normal GPUI owner before opening another.
        let _ = first_window.update(cx, |_, window, _| window.remove_window());
        application.update(cx, |app, cx| {
            app.open_floating_download(Some(url.into()), cx)
                .expect("prefill form")
        });
        let view = application.read_with(cx, |app, _| {
            app.new_download_view.clone().expect("prefill view")
        });
        assert_eq!(
            view.read_with(cx, |view, cx| view.url.read(cx).value().to_string()),
            url
        );
        assert_eq!(
            view.read_with(cx, |view, _| view
                .external_request
                .as_ref()
                .map(|r| r.source_url.clone())),
            Some(url.into()),
            "ordinary Add Download receives the URL as an initial draft"
        );
        assert!(
            view.read_with(cx, |view, _| view.probe.is_none()),
            "probe/confirmation have not silently submitted"
        );
        assert!(
            application.read_with(cx, |app, _| app.tasks.is_empty()),
            "prefill never submits"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
