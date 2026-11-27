use super::*;
use gpui_kit::TestAppContext;

fn request(directory: &Path, name: &str) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("https://example.test/{name}"),
        save_directory: directory.to_path_buf(),
        save_directory_explicit: false,
        file_name: Some(name.into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: Some(100),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    }
}

fn continuous_state_case(cx: &mut TestAppContext, cached_bytes: Option<u64>) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "continuous-progress");
    application.update(cx, |app, cx| {
        let events = app.engine.subscribe_events().expect("bounded event observer");
        let task_id = app.engine.enqueue(request(&directory, "first.bin"), RequestOrigin::Internal)
            .expect("queued task without network execution");
        app.engine.enqueue(request(&directory, "second.bin"), RequestOrigin::Internal)
            .expect("later engine state is already in the compact snapshot");
        let state = std::iter::from_fn(|| events.try_recv().ok())
            .find(|event| matches!(&event.event, DownloadEvent::TaskChanged { task } if task.id() == task_id))
            .expect("real earlier state event");
        let snapshot = load_download_snapshot(&app.engine).expect("real engine snapshot");
        assert!(snapshot.sequence > state.sequence, "engine is ahead of the consumer");
        let progress = DownloadProgress {
            task_id,
            downloaded_bytes: cached_bytes.unwrap_or(12),
            total_bytes: Some(100),
            bytes_per_second: 4,
            active_connections: 1,
            segments: Vec::new(),
            finishing: None,
        };
        app.event_generation = state.generation;
        app.last_event_sequence = state.sequence.saturating_sub(1);
        if cached_bytes.is_some() {
            app.latest_progress.insert(task_id, progress.clone());
        }
        let next_sequence = state.sequence + 1;
        app.handle_engine_event(state, cx);
        if let Some(bytes) = cached_bytes {
            let expected = (bytes == 0).then_some(&progress);
            assert_eq!(app.progress(task_id), expected,
                "ordinary refresh retains compatible progress and retires reset bytes");
        } else {
            assert_eq!(app.last_event_sequence, next_sequence - 1,
                "consuming state does not acknowledge later queued progress");
            app.handle_engine_event(DownloadEventEnvelope {
                generation: app.event_generation,
                sequence: next_sequence,
                emitted_at_unix_ms: 0,
                event: DownloadEvent::TaskProgress { progress: progress.clone() },
            }, cx);
            assert_eq!(app.progress(task_id), Some(&progress));
        }
    });
    std::fs::remove_dir_all(directory).expect("remove synthetic projection fixture");
}

#[gpui_kit::test]
fn continuous_state_snapshot_does_not_skip_queued_progress(cx: &mut TestAppContext) {
    continuous_state_case(cx, None);
}

#[gpui_kit::test]
fn continuous_state_snapshot_preserves_known_live_progress(cx: &mut TestAppContext) {
    continuous_state_case(cx, Some(0));
}

#[gpui_kit::test]
fn continuous_state_snapshot_discards_progress_after_bytes_reset(cx: &mut TestAppContext) {
    continuous_state_case(cx, Some(12));
}
