//! Identity-bound automatic browser ownership, separate from ordinary manual intake.
use super::*;
use nxtools_download::BrowserHandoffState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffOperation {
    handoff_id: Uuid,
    #[serde(default)]
    task_id: Option<Uuid>,
}

pub(super) fn prepare(
    request_id: String,
    payload: Value,
    paths: &AppPaths,
    executable: &Path,
) -> Result<BrowserResponse> {
    prepare_with_client(request_id, payload, paths, |mode, correlation_id| {
        connect_or_start(executable, mode, correlation_id)
    })
}

pub(super) fn prepare_with_client(
    request_id: String,
    payload: Value,
    paths: &AppPaths,
    connect: impl FnOnce(DesktopStartMode, Uuid) -> Result<DownloadIpcClient>,
) -> Result<BrowserResponse> {
    let payload: BrowserDownloadPayload = serde_json::from_value(payload)
        .context("controlled browser payload has an invalid shape")?;
    validate_payload(&payload)?;
    let handoff_id = Uuid::parse_str(&payload.handoff_id).context("handoffId must be a UUID")?;
    let correlation_id = correlation_id(&request_id)?;
    let started = Instant::now();
    let open_dialog = payload.open_dialog;
    let mode = if payload.start_immediately {
        DownloadSubmitMode::StartNow
    } else {
        DownloadSubmitMode::QueueOnly
    };
    let config = AppConfig::load_or_create(paths)?;
    let directory = if config.download.default_directory.trim().is_empty() {
        default_download_directory(paths)
    } else {
        PathBuf::from(&config.download.default_directory)
    };
    let request = build_browser_download_request(payload, &config, directory)?;
    let command = command_envelope(
        correlation_id,
        DownloadCommand::BrowserPrepareHandoff {
            handoff_id,
            request,
            open_dialog,
            mode,
        },
        RequestOrigin::Browser,
    );
    info!(event = "browser_native_host.controlled_handoff.started", %correlation_id, %handoff_id,
        open_dialog, "controlled browser preparation started");
    let result = connect(
        DesktopStartMode::for_browser_request(open_dialog),
        correlation_id,
    )
    .and_then(|mut client| {
        if open_dialog {
            command_browser_dialog_when_ready(&mut client, command)
        } else {
            client.command(command)
        }
    });
    match result {
        Ok(result) => project(request_id, handoff_id, None, result, started, true),
        Err(_) => Ok(browser_handoff_failure(
            request_id,
            handoff_id,
            "desktop_unavailable",
            started,
            None,
            None,
        )),
    }
}

pub(super) fn operate(request_id: String, kind: &str, payload: Value) -> Result<BrowserResponse> {
    operate_with_client(request_id, kind, payload, || {
        DownloadIpcClient::connect(DOWNLOAD_PIPE_NAME, Duration::from_millis(150))
    })
}

pub(super) fn operate_with_client(
    request_id: String,
    kind: &str,
    payload: Value,
    connect: impl FnOnce() -> Result<DownloadIpcClient>,
) -> Result<BrowserResponse> {
    let operation: HandoffOperation = serde_json::from_value(payload)
        .context("controlled browser operation has an invalid shape")?;
    let correlation_id = correlation_id(&request_id)?;
    let handoff_id = operation.handoff_id;
    let expected_task = operation.task_id.map(DownloadTaskId::from_uuid);
    let command = match kind {
        "get_browser_handoff" => DownloadCommand::BrowserHandoffStatus { handoff_id },
        "commit_browser_handoff" => DownloadCommand::BrowserCommitHandoff {
            handoff_id,
            task_id: expected_task.context("taskId is required for browser commit")?,
        },
        "abort_browser_handoff" => DownloadCommand::BrowserAbortHandoff { handoff_id },
        _ => bail!("unsupported controlled browser operation"),
    };
    let started = Instant::now();
    let result = connect().and_then(|mut client| {
        client.command(command_envelope(
            correlation_id,
            command,
            RequestOrigin::Browser,
        ))
    });
    match result {
        Ok(result) => project(
            request_id,
            handoff_id,
            expected_task,
            result,
            started,
            kind != "get_browser_handoff",
        ),
        Err(_) => Ok(browser_handoff_failure(
            request_id,
            handoff_id,
            "desktop_unavailable",
            started,
            None,
            None,
        )),
    }
}

fn project(
    request_id: String,
    expected_handoff: Uuid,
    expected_task: Option<DownloadTaskId>,
    result: DownloadCommandResult,
    started: Instant,
    log_transition: bool,
) -> Result<BrowserResponse> {
    if result.request_id != correlation_id(&request_id)? {
        return Ok(browser_handoff_failure(
            request_id,
            expected_handoff,
            "identity_mismatch",
            started,
            None,
            None,
        ));
    }
    let DownloadCommandOutcome::BrowserHandoff {
        handoff_id,
        state,
        task_id,
    } = result.outcome
    else {
        return Ok(browser_handoff_failure(
            request_id,
            expected_handoff,
            "desktop_refused",
            started,
            None,
            None,
        ));
    };
    if handoff_id != expected_handoff
        || (matches!(
            state,
            BrowserHandoffState::Held | BrowserHandoffState::Committed
        ) && task_id.is_none())
        || (expected_task.is_some() && task_id.is_some() && expected_task != task_id)
    {
        return Ok(browser_handoff_failure(
            request_id,
            expected_handoff,
            "identity_mismatch",
            started,
            None,
            None,
        ));
    }
    let (handoff_state, reason, message) = match state {
        BrowserHandoffState::Pending => (
            "awaiting_confirmation",
            "confirmation_pending",
            "原下载已暂停，等待 NxTools 确认",
        ),
        BrowserHandoffState::Held => (
            "persisted",
            "desktop_accepted",
            "桌面已持久接受，等待浏览器交接",
        ),
        BrowserHandoffState::Committed => {
            ("transferred", "browser_transferred", "浏览器交接已完成")
        }
        BrowserHandoffState::Aborted => (
            "rejected",
            "confirmation_cancelled",
            "已取消桌面接管，保留浏览器下载",
        ),
        BrowserHandoffState::Expired => (
            "rejected",
            "confirmation_timeout",
            "桌面确认已过期，保留浏览器下载",
        ),
        BrowserHandoffState::Missing => (
            "rejected",
            "desktop_unavailable",
            "桌面接管已失效，保留浏览器下载",
        ),
    };
    if log_transition {
        info!(event = "browser_native_host.controlled_handoff.transition", correlation_id = %result.request_id,
            %handoff_id, ?task_id, handoff_state, reason, elapsed_ms = started.elapsed().as_millis(),
            "controlled browser handoff transition");
    }
    let mut response = BrowserResponse::ok(
        request_id,
        Some(json!({
            "browserHandoffState": state, "handoffState": handoff_state, "reason": reason,
            "queued": state == BrowserHandoffState::Held,
        })),
    );
    response.handoff_id = Some(handoff_id.to_string());
    response.task_id = task_id.map(|value| value.as_uuid().to_string());
    response.accepted = matches!(
        state,
        BrowserHandoffState::Held | BrowserHandoffState::Committed
    );
    response.message = Some(message.into());
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controlled_projection_requires_exact_result_handoff_and_task_identities() {
        let handoff = Uuid::new_v4();
        let task = DownloadTaskId::from_uuid(Uuid::new_v4());
        let outcome = |handoff_id, task_id| DownloadCommandOutcome::BrowserHandoff {
            handoff_id,
            state: BrowserHandoffState::Held,
            task_id,
        };
        for (request, id, task_id) in [
            (Uuid::new_v4(), handoff, Some(task)),
            (handoff, Uuid::new_v4(), Some(task)),
            (handoff, handoff, None),
            (
                handoff,
                handoff,
                Some(DownloadTaskId::from_uuid(Uuid::new_v4())),
            ),
        ] {
            let result = DownloadCommandResult {
                request_id: request,
                outcome: outcome(id, task_id),
            };
            let reply = project(
                handoff.to_string(),
                handoff,
                Some(task),
                result,
                Instant::now(),
                false,
            )
            .expect("fixed identity refusal");
            assert!(!reply.accepted);
            assert_eq!(
                reply.data.expect("fixed metadata")["reason"],
                "identity_mismatch"
            );
        }
    }

    #[test]
    fn controlled_frames_keep_pending_distinct_from_durable_acceptance() {
        let handoff = Uuid::new_v4();
        let task = DownloadTaskId::from_uuid(Uuid::new_v4());
        for state in [
            BrowserHandoffState::Pending,
            BrowserHandoffState::Held,
            BrowserHandoffState::Committed,
            BrowserHandoffState::Aborted,
            BrowserHandoffState::Expired,
            BrowserHandoffState::Missing,
        ] {
            let task_id = matches!(
                state,
                BrowserHandoffState::Held | BrowserHandoffState::Committed
            )
            .then_some(task);
            let result = DownloadCommandResult {
                request_id: handoff,
                outcome: DownloadCommandOutcome::BrowserHandoff {
                    handoff_id: handoff,
                    state,
                    task_id,
                },
            };
            let response = project(
                handoff.to_string(),
                handoff,
                None,
                result,
                Instant::now(),
                false,
            )
            .expect("state projection");
            let mut bytes = Vec::new();
            write_native_message(&mut bytes, &response).expect("native frame");
            let raw = read_native_message(&mut bytes.as_slice())
                .expect("bounded frame")
                .expect("response");
            let reply: Value = serde_json::from_slice(&raw).expect("reply shape");
            assert_eq!(reply["handoffId"], handoff.to_string());
            assert_eq!(
                reply["accepted"],
                matches!(
                    state,
                    BrowserHandoffState::Held | BrowserHandoffState::Committed
                )
            );
            assert_eq!(
                reply["data"]["browserHandoffState"],
                serde_json::to_value(state).expect("state")
            );
        }
    }
}

#[cfg(test)]
#[path = "controlled_handoff_tests.rs"]
mod lifecycle_tests;
