use super::*;

fn assert_retained(
    engine: &DownloadEngine,
    store: &SqliteDownloadStore,
    paused: &nxtools_download::DownloadEngineTask,
    plan: &nxtools_download::TransferPlan,
    bytes: &[u8],
) {
    let current = engine
        .task(paused.task.id())
        .expect("current task")
        .expect("retained identity");
    assert_eq!(current.task, paused.task);
    assert_eq!(current.probe, paused.probe);
    assert_eq!(current.downloaded_bytes, paused.downloaded_bytes);
    assert_eq!(current.temporary_path, paused.temporary_path);
    assert_eq!(
        store
            .load_task(paused.task.id())
            .expect("persisted task")
            .as_ref(),
        Some(&paused.task)
    );
    assert_eq!(
        store
            .load_transfer_plan(paused.task.id())
            .expect("persisted plan")
            .as_ref(),
        Some(plan)
    );
    assert_eq!(
        std::fs::read(paused.temporary_path.as_ref().expect("artifact")).expect("retained bytes"),
        bytes
    );
    assert!(
        store
            .is_queue_task_user_paused(paused.task.id())
            .expect("user pause")
    );
    assert!(
        !store
            .is_queue_task_eligible(paused.task.id())
            .expect("not eligible")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_identity_and_probe_refusals_preserve_original_resume() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("original engine");
    let old = server.url("/old");
    let mut original = request(old.clone(), directory.path());
    original
        .headers
        .push(nxtools_download::DownloadRequestHeader {
            name: "User-Agent".into(),
            value: "Retained-Agent/1".into(),
        });
    original.cookie = Some("retained=1".into());
    original.referrer = Some("https://retained.invalid/page".into());
    let task_id = engine
        .submit(original, RequestOrigin::Ui)
        .expect("original admission");
    let paused = pause_written(&engine, task_id).await;
    let store = SqliteDownloadStore::open(&database).expect("SQLite");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("written plan");
    let bytes = std::fs::read(paused.temporary_path.as_ref().expect("artifact")).expect("bytes");
    for path in ["/changed", "/missing", "/weak", "/no-range", "/failed"] {
        let result = replacement(&engine, task_id, &old, &server.url(path), Uuid::new_v4());
        assert!(
            matches!(result.outcome, DownloadCommandOutcome::Rejected { .. }),
            "refuse candidate {path}"
        );
        assert_retained(&engine, &store, &paused, &plan, &bytes);
    }
    engine.resume(task_id).expect("original still resumes");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("old source completion");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(directory.path().join("retained.bin")).expect("original bytes"),
        url_server::CONTENT
    );
    drop(store);
    drop(engine);
    let requests = server.close().await;
    let candidates: Vec<_> = requests
        .iter()
        .filter(|request| {
            !request
                .lines()
                .next()
                .expect("HTTP request")
                .contains(" /old ")
        })
        .collect();
    assert!(candidates.len() >= 5);
    for request in candidates {
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains("user-agent: retained-agent/1\r\n"));
        assert!(lower.contains("cookie: retained=1\r\n"));
        assert!(lower.contains("referer: https://retained.invalid/page\r\n"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_equal_empty_or_weak_validators_do_not_prove_identity() {
    for (old_path, new_path) in [("/old-missing", "/missing"), ("/old-weak", "/weak")] {
        let server = url_server::UrlServer::start().await;
        let directory = tempfile::tempdir().expect("owned output");
        let database = directory.path().join("downloads.sqlite3");
        let engine = DownloadEngine::open(&database).expect("engine");
        let old = server.url(old_path);
        let task_id = engine
            .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
            .expect("submit");
        let paused = pause_written(&engine, task_id).await;
        let store = SqliteDownloadStore::open(&database).expect("store");
        let plan = store
            .load_transfer_plan(task_id)
            .expect("plan")
            .expect("written plan");
        let bytes =
            std::fs::read(paused.temporary_path.as_ref().expect("artifact")).expect("bytes");
        let result = replacement(
            &engine,
            task_id,
            &old,
            &server.url(new_path),
            Uuid::new_v4(),
        );
        assert!(
            matches!(result.outcome, DownloadCommandOutcome::Rejected { .. }),
            "equal non-identity validator must be refused"
        );
        assert_retained(&engine, &store, &paused, &plan, &bytes);
        drop(store);
        drop(engine);
        server.close().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_original_last_modified_policy_can_authorize_replacement() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let old = server.url("/last-old");
    let task_id = engine
        .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
        .expect("submit");
    let paused = pause_written(&engine, task_id).await;
    assert!(
        paused
            .probe
            .as_ref()
            .expect("probe")
            .validator
            .etag
            .is_none()
    );
    assert!(matches!(
        replacement(
            &engine,
            task_id,
            &old,
            &server.url("/last-new"),
            Uuid::new_v4()
        )
        .outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
    engine.resume(task_id).expect("resume");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("terminal");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(directory.path().join("retained.bin")).expect("bytes"),
        url_server::CONTENT
    );
    drop(engine);
    let requests = server.close().await;
    assert!(requests.iter().any(|request| {
        request
            .to_ascii_lowercase()
            .contains("if-range: mon, 27 jul 2026 00:00:00 gmt\r\n")
    }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_sqlite_failure_rolls_back_request_probe_plan_and_publication() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("engine");
    let old = server.url("/old");
    let task_id = engine
        .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
        .expect("submit");
    let paused = pause_written(&engine, task_id).await;
    let store = SqliteDownloadStore::open(&database).expect("store");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("written plan");
    let bytes = std::fs::read(paused.temporary_path.as_ref().expect("artifact")).expect("bytes");
    let events = engine.subscribe_events().expect("original event observer");
    let connection = rusqlite::Connection::open(&database).expect("owned failure injection");
    connection.execute_batch("CREATE TRIGGER b4_ref08_fail_plan BEFORE UPDATE OF transfer_plan_json ON download_tasks BEGIN SELECT RAISE(ABORT, 'owned replacement failure'); END;").expect("inject final transaction write failure");
    assert!(matches!(
        replacement(&engine, task_id, &old, &server.url("/same"), Uuid::new_v4()).outcome,
        DownloadCommandOutcome::Rejected { .. }
    ));
    assert_retained(&engine, &store, &paused, &plan, &bytes);
    while let Ok(event) = events.try_recv() {
        if let nxtools_download::DownloadEvent::TaskChanged { task } = event.event {
            assert_eq!(
                task.request().source_url,
                old,
                "failed candidate never publishes the new URL"
            );
        }
    }
    connection
        .execute_batch("DROP TRIGGER b4_ref08_fail_plan;")
        .expect("remove owned injection");
    assert!(matches!(
        replacement(&engine, task_id, &old, &server.url("/same"), Uuid::new_v4()).outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
    let mut committed_events = 0;
    while let Ok(event) = events.try_recv() {
        if let nxtools_download::DownloadEvent::TaskChanged { task } = event.event
            && task.request().source_url == server.url("/same")
        {
            committed_events += 1;
        }
    }
    assert_eq!(
        committed_events, 1,
        "one accepted source event after SQLite commit"
    );
    drop(connection);
    drop(store);
    drop(engine);
    let reopened = DownloadEngine::open(&database).expect("durable reopen");
    assert_eq!(
        reopened
            .task(task_id)
            .expect("task")
            .expect("identity")
            .task
            .request()
            .source_url,
        server.url("/same")
    );
    reopened.resume(task_id).expect("resume");
    assert_eq!(
        reopened
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("terminal")
            .task
            .status(),
        DownloadStatus::Completed
    );
    drop(reopened);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_cancel_drains_probe_and_excludes_duplicate_and_resume() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("engine");
    let old = server.url("/old");
    let new = server.url("/held");
    let task_id = engine
        .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
        .expect("submit");
    let paused = pause_written(&engine, task_id).await;
    let store = SqliteDownloadStore::open(&database).expect("store");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("written plan");
    let bytes = std::fs::read(paused.temporary_path.as_ref().expect("artifact")).expect("bytes");
    let operation_id = Uuid::new_v4();
    let worker_engine = engine.clone();
    let worker_old = old.clone();
    let worker_new = new.clone();
    let worker = tokio::task::spawn_blocking(move || {
        replacement(
            &worker_engine,
            task_id,
            &worker_old,
            &worker_new,
            operation_id,
        )
    });
    server.wait_held().await;
    for duplicate_id in [operation_id, Uuid::new_v4()] {
        assert!(matches!(
            replacement(&engine, task_id, &old, &new, duplicate_id).outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert!(
            engine.resume(task_id).is_err(),
            "duplicate rejection cannot retire the active lease"
        );
    }
    assert!(
        !store
            .is_queue_task_eligible(task_id)
            .expect("resume did not admit")
    );
    let cancelled = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::CancelPausedUrlReplacement {
            task_id,
            operation_id,
        },
    });
    assert!(matches!(
        cancelled.outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
    let result = tokio::time::timeout(Duration::from_secs(3), worker)
        .await
        .expect("candidate physical completion")
        .expect("worker");
    assert!(
        matches!(result.outcome, DownloadCommandOutcome::Rejected { fault } if fault.code == nxtools_download::DownloadFaultCode::Cancelled)
    );
    server.wait_drained().await;
    assert_retained(&engine, &store, &paused, &plan, &bytes);
    server
        .hold
        .store(false, std::sync::atomic::Ordering::Release);
    assert!(
        matches!(
            replacement(&engine, task_id, &old, &new, operation_id).outcome,
            DownloadCommandOutcome::Rejected { .. }
        ),
        "cancelled confirmation cannot be replayed"
    );
    engine
        .resume(task_id)
        .expect("resume after candidate drain");
    assert_eq!(
        engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("terminal")
            .task
            .status(),
        DownloadStatus::Completed
    );
    drop(store);
    drop(engine);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_precancel_deadline_and_stale_generation_keep_original() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let old = server.url("/old");
    let task_id = engine
        .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
        .expect("submit");
    let paused = pause_written(&engine, task_id).await;
    let command = |generation| nxtools_download::DownloadPausedUrlReplacement {
        task_id,
        operation_id: Uuid::new_v4(),
        generation,
        expected_revision: engine
            .paused_url_replacement_revision(task_id)
            .expect("current attempt revision"),
        expected_source_url: old.clone(),
        source_url: server.url("/held"),
    };
    let before = server.requests.lock().expect("requests").len();
    let control = nxtools_download::DownloadResolutionControl::default();
    control.cancel();
    assert!(
        engine
            .replace_paused_url_with_control(
                command(engine.session_generation()),
                Uuid::new_v4(),
                control
            )
            .is_err()
    );
    assert!(
        engine
            .replace_paused_url_with_control(
                command(engine.session_generation().wrapping_add(1)),
                Uuid::new_v4(),
                Default::default()
            )
            .is_err()
    );
    assert_eq!(
        server.requests.lock().expect("requests").len(),
        before,
        "pre-cancel/stale generation performs no HTTP"
    );
    let replacement = command(engine.session_generation());
    let worker_engine = engine.clone();
    let worker = tokio::task::spawn_blocking(move || {
        worker_engine.replace_paused_url_with_control(
            replacement,
            Uuid::new_v4(),
            nxtools_download::DownloadResolutionControl::with_timeout(Duration::from_millis(80)),
        )
    });
    server.wait_held().await;
    assert!(worker.await.expect("physical operation").is_err());
    server.wait_drained().await;
    server
        .hold
        .store(false, std::sync::atomic::Ordering::Release);
    assert_eq!(
        engine
            .task(task_id)
            .expect("task")
            .expect("same identity")
            .task,
        paused.task
    );
    engine.resume(task_id).expect("deadline released lease");
    assert_eq!(
        engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("terminal")
            .task
            .status(),
        DownloadStatus::Completed
    );
    drop(engine);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_paused_queue_admission_and_recovery_are_not_explicit_pause() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("engine");
    let old = server.url("/old");
    let task_id = engine
        .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
        .expect("submit");
    let paused = pause_written(&engine, task_id).await;
    let store = SqliteDownloadStore::open(&database).expect("store");
    let before = server.requests.lock().expect("requests").len();
    for (eligible, user_paused) in [(true, false), (false, false)] {
        store
            .set_queue_task_admission(task_id, eligible, user_paused)
            .expect("original persisted admission boundary");
        assert!(matches!(
            replacement(&engine, task_id, &old, &server.url("/same"), Uuid::new_v4()).outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert_eq!(
            engine.task(task_id).expect("task").expect("identity").task,
            paused.task
        );
    }
    assert_eq!(
        server.requests.lock().expect("requests").len(),
        before,
        "ineligible replacement performs no probe"
    );
    store
        .set_queue_task_admission(task_id, false, true)
        .expect("restore explicit pause");
    assert!(matches!(
        replacement(&engine, task_id, &old, &server.url("/same"), Uuid::new_v4()).outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
    drop(store);
    drop(engine);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_task_cancel_or_remove_retires_candidate_before_late_reply() {
    for remove in [false, true] {
        let server = url_server::UrlServer::start().await;
        let directory = tempfile::tempdir().expect("owned output");
        let engine = DownloadEngine::open_in_memory().expect("engine");
        let old = server.url("/old");
        let new = server.url("/held");
        let task_id = engine
            .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
            .expect("submit");
        pause_written(&engine, task_id).await;
        let worker_engine = engine.clone();
        let worker_old = old.clone();
        let worker = tokio::task::spawn_blocking(move || {
            replacement(&worker_engine, task_id, &worker_old, &new, Uuid::new_v4())
        });
        server.wait_held().await;
        if remove {
            engine.remove(task_id, false).expect("original remove");
        } else {
            engine.cancel(task_id).expect("original task cancel");
        }
        assert!(matches!(
            worker.await.expect("physical completion").outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        server.wait_drained().await;
        server
            .hold
            .store(false, std::sync::atomic::Ordering::Release);
        let current = engine.task(task_id).expect("task read");
        if remove {
            assert!(current.is_none());
        } else {
            let current = current.expect("cancelled task retained");
            assert_eq!(current.task.status(), DownloadStatus::Cancelled);
            assert_eq!(current.task.request().source_url, old);
        }
        drop(engine);
        server.close().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_repair01_validator_selected_for_replacement_is_used_by_actual_resume() {
    for kind in ["blank-last", "space-last", "quoted-empty", "weak-last"] {
        let server = url_server::UrlServer::start().await;
        let directory = tempfile::tempdir().expect("owned validator output");
        let database = directory.path().join("downloads.sqlite3");
        let engine = DownloadEngine::open(&database).expect("original SQLite engine");
        let old = server.url(&format!("/{kind}-old"));
        let new = server.url(&format!("/{kind}-new"));
        let task_id = engine
            .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
            .expect("real writer");
        let paused = pause_written(&engine, task_id).await;
        let store = SqliteDownloadStore::open(&database).expect("original store");
        let plan = store
            .load_transfer_plan(task_id)
            .expect("plan")
            .expect("written plan");
        let artifact = paused.temporary_path.clone().expect("partial artifact");
        let bytes = std::fs::read(&artifact).expect("written bytes");
        let accepted = replacement(&engine, task_id, &old, &new, Uuid::new_v4());
        assert!(
            matches!(accepted.outcome, DownloadCommandOutcome::Accepted { .. }),
            "same effective validator accepts {kind}: {:?}",
            accepted.outcome
        );
        assert_eq!(
            store
                .load_transfer_plan(task_id)
                .expect("committed plan")
                .expect("plan")
                .segments(),
            plan.segments()
        );
        assert_eq!(std::fs::read(&artifact).expect("retained prefix"), bytes);
        engine.resume(task_id).expect("original resume");
        let completed = engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("writer drain");
        let requests = server.close().await;
        let payload: Vec<_> = requests
            .iter()
            .filter(|request| {
                request.starts_with(&format!("GET /{kind}-new "))
                    && !request
                        .to_ascii_lowercase()
                        .contains("range: bytes=0-0\r\n")
            })
            .collect();
        let expected = if kind == "quoted-empty" {
            "\"\""
        } else {
            "Mon, 27 Jul 2026 00:00:00 GMT"
        };
        assert!(!payload.is_empty(), "actual suffix transfer");
        for captured in &payload {
            let actual = captured.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("if-range").then(|| value.trim())
            });
            assert_eq!(
                actual,
                Some(expected),
                "accepted identity must be the actual If-Range for {kind}"
            );
            assert!(
                captured.to_ascii_lowercase().contains(&format!(
                    "range: bytes={}-{}\r\n",
                    paused.downloaded_bytes.saturating_sub(64),
                    url_server::CONTENT.len() - 1
                )),
                "resume verifies 64 retained bytes before the suffix, never a fresh restart"
            );
        }
        assert_eq!(
            payload.len(),
            1,
            "one retained-suffix transfer, no implicit fresh restart"
        );
        assert_eq!(
            completed.task.status(),
            DownloadStatus::Completed,
            "{:?}",
            completed.error
        );
        assert_eq!(completed.task.id(), task_id);
        assert_eq!(
            std::fs::read(directory.path().join("retained.bin")).expect("exact final bytes"),
            url_server::CONTENT
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_repair02_retired_attempt_cannot_replay_after_another_failure() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned replay output");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("original engine");
    let old = server.url("/old");
    let task_id = engine
        .submit(request(old.clone(), directory.path()), RequestOrigin::Ui)
        .expect("original admission");
    let paused = pause_written(&engine, task_id).await;
    let store = SqliteDownloadStore::open(&database).expect("original store");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("written plan");
    let bytes =
        std::fs::read(paused.temporary_path.as_ref().expect("artifact")).expect("written bytes");
    let attempt_id = Uuid::new_v4();
    let attempt_a = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::ReplacePausedUrl(
            nxtools_download::DownloadPausedUrlReplacement {
                task_id,
                operation_id: attempt_id,
                generation: engine.session_generation(),
                expected_revision: engine
                    .paused_url_replacement_revision(task_id)
                    .expect("A revision frozen in envelope"),
                expected_source_url: old.clone(),
                source_url: server.url("/held"),
            },
        ),
    };
    let worker_engine = engine.clone();
    let first = attempt_a.clone();
    let worker = tokio::task::spawn_blocking(move || worker_engine.handle_command(first));
    server.wait_held().await;
    let cancel = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::CancelPausedUrlReplacement {
            task_id,
            operation_id: attempt_id,
        },
    });
    assert!(matches!(
        cancel.outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
    let retired = tokio::time::timeout(Duration::from_secs(3), worker)
        .await
        .expect("A physically completes")
        .expect("A worker");
    assert!(
        matches!(retired.outcome, DownloadCommandOutcome::Rejected { fault } if fault.code == nxtools_download::DownloadFaultCode::Cancelled)
    );
    server.wait_drained().await;
    assert_retained(&engine, &store, &paused, &plan, &bytes);
    let attempt_b = replacement(
        &engine,
        task_id,
        &old,
        &server.url("/failed"),
        Uuid::new_v4(),
    );
    assert!(matches!(
        attempt_b.outcome,
        DownloadCommandOutcome::Rejected { .. }
    ));
    assert_retained(&engine, &store, &paused, &plan, &bytes);
    server
        .hold
        .store(false, std::sync::atomic::Ordering::Release);
    let requests_before = server.requests.lock().expect("HTTP facts").len();
    let replay = engine.handle_command(attempt_a);
    assert!(
        matches!(replay.outcome, DownloadCommandOutcome::Rejected { .. }),
        "retired A must remain rejected after B: {:?}",
        replay.outcome
    );
    assert_eq!(
        server.requests.lock().expect("HTTP facts").len(),
        requests_before,
        "retired A rejects before HTTP"
    );
    assert_retained(&engine, &store, &paused, &plan, &bytes);
    let fresh = replacement(&engine, task_id, &old, &server.url("/same"), Uuid::new_v4());
    assert!(
        matches!(fresh.outcome, DownloadCommandOutcome::Accepted { .. }),
        "fresh C remains usable"
    );
    assert_eq!(
        store
            .load_transfer_plan(task_id)
            .expect("C plan")
            .expect("plan")
            .segments(),
        plan.segments()
    );
    engine
        .resume(task_id)
        .expect("fresh C retains original resume");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("writer drain");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(directory.path().join("retained.bin")).expect("exact output"),
        url_server::CONTENT
    );
    drop(store);
    drop(engine);
    server.close().await;
}
