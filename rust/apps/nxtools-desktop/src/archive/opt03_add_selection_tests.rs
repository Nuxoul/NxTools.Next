use super::*;
use nxtools_protocol::archive_job::*;

fn entry(ordinal: u64, name: &str) -> nxtools_protocol::archive_read::ArchiveReadEntry {
    serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":"File","size":1,
        "packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("catalog entry")
}
fn add_case(cx: &mut TestAppContext, count: usize, stale: Option<&str>) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let generation = owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "Tar".into();
        browser.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = browser.clone();
        slot.window
            .update(cx, |_, window, cx| {
                slot.view.update(cx, |view, cx| {
                    view.install(
                        snapshot,
                        vec![entry(71, "a/skipped.txt"), entry(72, "a/overwritten.txt")],
                        "Tar".into(),
                        window,
                        cx,
                    );
                    view.bind_edit_context(&browser, cx);
                    let model = view.model.as_mut().expect("model");
                    model.enter("a");
                    model.select(
                        crate::archive::model::RowId::Entry(snapshot, 71),
                        false,
                        false,
                    );
                })
            })
            .expect("window")
            .expect("view");
        slot.generation
    });
    let job = owner.update(cx, |app, cx| {
        let browser = app.windows[&id].browser.clone().expect("context");
        let request: ArchiveJobRequest = serde_json::from_value(serde_json::json!({
            "kind":"modify", "paths":["C:\\owned\\preview.zip", "C:\\owned\\incoming"],
            "modify":{"browser":browser,"action":"add","ordinals":[],"value":"a/"}
        }))
        .expect("Add request");
        let job = app
            .submit_job(
                request,
                crate::archive::forms::FormCompletion::default(),
                cx,
            )
            .expect("Add admission");
        app.jobs.records.get_mut(&job).expect("record").edit_origin =
            Some((id, generation, snapshot));
        job
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    if matches!(stale, Some("folder" | "snapshot")) {
        owner.update(cx, |app, cx| {
            app.windows[&id]
                .view
                .update(cx, |view, _| {
                    let model = view.model.as_mut().expect("model");
                    if stale == Some("folder") {
                        model.folder = "elsewhere".into();
                    } else {
                        model.snapshot = Uuid::new_v4();
                    }
                })
                .expect("navigation")
        });
    }
    let names = (0..count)
        .map(|index| match index {
            0 => "a/overwritten.txt".into(),
            1 => "a/renamed (2).txt".into(),
            2 => "a/tree/child.txt".into(),
            _ => format!("a/new-{index}.txt"),
        })
        .collect::<Vec<String>>();
    let shown = names
        .iter()
        .take(256)
        .enumerate()
        .map(|(index, name)| {
            serde_json::json!({
                "path":name, "outcome":match index { 0=>"overwritten",1=>"renamed",_=>"added" }
            })
        })
        .collect::<Vec<_>>();
    let mut facts: ArchiveJobTerminal = serde_json::from_value(serde_json::json!({
        "outcome":"succeeded","files_committed":1,"bytes_committed":100,
        "files_verified":count,"bytes_verified":count,"files_skipped":1,
        "cleanup_complete":true,"commit_state":"Committed","elapsed_ms":1,
        "journal":[{"ordinal":0,"state":"Committed","replaced":true,"bytes":100,"published":true,"cleanup":"retained"}],
        "output_published":true,"add_result":{"total":count,"members":shown}
    })).expect("bounded result wire");
    if matches!(
        stale,
        Some("all_skip" | "unpublished" | "unknown_publication" | "unknown_commit" | "cancelled")
    ) {
        facts.files_committed = 0;
        facts.bytes_committed = 0;
        facts.output_published = Some(false);
        facts.commit_state = None;
        facts.add_result = None;
        facts.journal.clear();
    }
    match stale {
        Some("all_skip") => facts.files_skipped = count as u64,
        Some("unpublished") => {
            facts.outcome = ArchiveJobOutcome::Failed;
            facts.code = Some("source_changed".into());
            facts.commit_state = Some("FailedBeforeCommit".into());
        }
        Some("unknown_publication") => {
            facts.outcome = ArchiveJobOutcome::Failed;
            facts.code = Some("io_failure".into());
            facts.output_published = None;
        }
        Some("unknown_commit") => {
            facts.outcome = ArchiveJobOutcome::Failed;
            facts.code = Some("io_failure".into());
            facts.commit_state = Some("CommitOutcomeUnknown".into());
            facts.output_published = None;
            facts.journal.push(ArchiveJobCommit {
                ordinal: 0,
                state: "CommitOutcomeUnknown".into(),
                replaced: false,
                bytes: 100,
                native_identity: None,
                published: None,
                cleanup: Some("retained_uncertain".into()),
            });
        }
        Some("unclean") => {
            facts.outcome = ArchiveJobOutcome::Failed;
            facts.code = Some("cleanup_failed".into());
            facts.cleanup_complete = false;
            facts.add_result = None;
            facts.journal[0].cleanup = Some("cleanup_failed".into());
        }
        Some("cancelled") => {
            facts.outcome = ArchiveJobOutcome::Cancelled;
            facts.code = Some("cancelled".into());
        }
        Some("late_cancel") => {
            facts.outcome = ArchiveJobOutcome::Cancelled;
            facts.add_result = None;
        }
        Some("malformed") => facts.output_published = Some(false),
        _ => {}
    }
    let event = ArchiveJobEvent {
        version: 1,
        operation_id: if stale == Some("attempt") {
            Uuid::new_v4()
        } else {
            job
        },
        session_id: job,
        generation: if stale == Some("generation") { 2 } else { 1 },
        kind: ArchiveJobEventKind::Terminal,
        state: None,
        progress: None,
        prompt: None,
        terminal: Some(facts),
    };
    if stale == Some("malformed") {
        assert!(
            event.validate().is_err(),
            "contradictory Add result is a malformed-frame test only"
        );
    } else {
        assert!(
            event.validate().is_ok(),
            "valid terminal must reach the business boundary: {stale:?}"
        );
    }
    let expected_terminal = event.terminal.clone();
    let notification =
        PlatformNotification::Event(nxtools_protocol::PlatformEvent::ArchiveJob(Box::new(event)));
    endpoint
        .send_notification_for_contract_test(notification.clone())
        .expect("original IPC terminal");
    owner.update(cx, |app, cx| app.notification(notification, cx));
    cx.run_until_parked();
    if stale == Some("malformed") {
        assert!(owner.read_with(cx, |app, _| app.jobs.records[&job].terminal.is_none()));
    } else if !matches!(stale, Some("attempt" | "generation")) {
        owner.read_with(cx, |app, _| {
            assert_eq!(
                app.jobs.records[&job].terminal, expected_terminal,
                "matching terminal installed before refresh assertions"
            );
        });
    }
    let mut reads = vec![];
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(command)) = wire.message
            && command.action == ArchiveReadAction::Open
        {
            reads.push(command);
        }
    }
    if let Some(stale) = stale.filter(|state| *state != "late_cancel") {
        assert!(
            reads.is_empty(),
            "stale {stale} must not refresh or retarget the browser"
        );
        if !matches!(stale, "folder" | "snapshot") {
            owner.read_with(cx, |app, cx| {
                app.windows[&id]
                    .view
                    .read_with(cx, |view, _| {
                        let model = view.model.as_ref().expect("original model");
                        assert_eq!(model.snapshot, snapshot);
                        assert_eq!(model.folder, "a");
                        assert!(
                            model
                                .selected
                                .contains(&crate::archive::model::RowId::Entry(snapshot, 71))
                        );
                        assert!(view.add_result_summary.is_none());
                    })
                    .expect("unchanged view")
            });
        }
        return;
    }
    assert_eq!(reads.len(), 1, "one refresh of the original browser");
    let read = reads.pop().expect("refresh");
    deliver(
        &endpoint,
        read_event(&read, ArchiveReadEventKind::Accepted),
        cx,
    );
    let mut negotiated = read_event(&read, ArchiveReadEventKind::Negotiated);
    negotiated.capabilities = archive_read_capabilities(2)
        .iter()
        .map(|value| (*value).into())
        .collect();
    deliver(&endpoint, negotiated, cx);
    let fresh = Uuid::new_v4();
    let mut entries = vec![entry(0, "a/skipped.txt")];
    entries.extend(
        names
            .iter()
            .enumerate()
            .map(|(index, name)| entry(index as u64 + 1, name)),
    );
    let entries_count = entries.len() as u64;
    let batch_count = entries.len().div_ceil(256) as u64;
    for (sequence, chunk) in entries.chunks(256).enumerate() {
        let mut batch = read_event(&read, ArchiveReadEventKind::Batch);
        batch.batch = Some(ArchiveReadBatch {
            snapshot_id: fresh,
            sequence: sequence as u64,
            format: "Tar".into(),
            entries: chunk.to_vec(),
        });
        deliver(&endpoint, batch, cx);
    }
    let mut browser = owner.read_with(cx, |app, _| {
        app.jobs.records[&job]
            .command
            .request
            .as_ref()
            .expect("request")
            .modify
            .as_ref()
            .expect("Add")
            .browser
            .clone()
    });
    browser.snapshot_id = fresh;
    browser.source.digest = [2; 32];
    let mut finished = read_event(&read, ArchiveReadEventKind::Terminal);
    finished.terminal = Some(ArchiveReadTerminal {
        status: ArchiveReadStatus::Succeeded,
        code: None,
        files_verified: 0,
        bytes_verified: 0,
        entry_count: entries_count,
        batch_count,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: Some(ArchiveReadTransfer {
            private_bytes: 262144,
            received_bytes: 131072,
            max_batch_entries: entries_count.min(256),
            max_batch_bytes: 131072,
            max_inflight_batches: 1,
            process_budget_bytes: 536870912,
        }),
        browser: Some(Box::new(browser)),
    });
    deliver(&endpoint, finished, cx);
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        app.windows[&id]
            .view
            .read_with(cx, |view, _| {
                let model = view.model.as_ref().expect("installed model");
                assert_eq!(model.snapshot, fresh);
                assert_eq!(model.folder, "a");
                assert_eq!(
                    model.entries.len(),
                    count + 1,
                    "display budget never truncates the installed catalog"
                );
                if stale == Some("late_cancel") {
                    assert!(
                        model.selected.is_empty(),
                        "late cancellation has no invented success selection"
                    );
                    assert!(view.add_result_summary.is_none());
                    return;
                }
                let summary = view
                    .add_result_summary
                    .as_deref()
                    .expect("visible result summary");
                assert!(summary.contains(&format!("已更新 {count} 个成员")));
                if count > 256 {
                    assert!(summary.contains("展示其中 256 个结果"));
                }
                let selected = model
                    .visible()
                    .iter()
                    .filter(|row| model.selected.contains(&row.id))
                    .map(|row| row.path.clone())
                    .collect::<std::collections::HashSet<_>>();
                let expected = names
                    .iter()
                    .take(256)
                    .map(|name| {
                        if name == "a/tree/child.txt" {
                            "a/tree".into()
                        } else {
                            name.clone()
                        }
                    })
                    .collect();
                assert_eq!(
                    selected, expected,
                    "only actual results map to new snapshot IDs; Skip stays unselected"
                );
                assert!(
                    !model
                        .selected
                        .contains(&crate::archive::model::RowId::Entry(snapshot, 71))
                );
            })
            .expect("view")
    });
}
#[gpui_kit::test]
fn opt03_add_actual_results_select_new_snapshot(cx: &mut TestAppContext) {
    add_case(cx, 4, None);
}
#[gpui_kit::test]
fn opt03_add_display_budget_does_not_truncate_catalog(cx: &mut TestAppContext) {
    add_case(cx, 300, None);
}
#[gpui_kit::test]
fn opt03_add_stale_completion_does_not_retarget(cx: &mut TestAppContext) {
    for stale in ["attempt", "generation", "folder", "snapshot"] {
        add_case(cx, 4, Some(stale));
    }
}

#[gpui_kit::test]
fn opt03_add_unconfirmed_or_unchanged_terminal_preserves_browser(cx: &mut TestAppContext) {
    for terminal in [
        "all_skip",
        "unpublished",
        "unknown_publication",
        "unclean",
        "unknown_commit",
        "cancelled",
    ] {
        add_case(cx, 4, Some(terminal));
    }
}

#[gpui_kit::test]
fn opt03_add_published_late_cancel_refreshes_without_success_selection(cx: &mut TestAppContext) {
    add_case(cx, 4, Some("late_cancel"));
}

#[gpui_kit::test]
fn opt03_ac02_malformed_add_result_is_rejected_before_terminal_install(cx: &mut TestAppContext) {
    add_case(cx, 4, Some("malformed"));
}
