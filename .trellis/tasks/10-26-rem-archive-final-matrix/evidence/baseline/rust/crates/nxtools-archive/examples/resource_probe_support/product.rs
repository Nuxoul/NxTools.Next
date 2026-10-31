//! The actual typed product fixture. All stop paths retire the ORIGINAL operation.
use super::{
    Fixture, POLL, ProbeStatus,
    control::{exact_image, read_bounded},
};
use nxtools_archive::product::{ArchiveReadClient, ArchiveReadOperation};
use nxtools_ipc::{
    PlatformBridge, PlatformBridgeError, PlatformServerEndpoint, platform_bridge,
    run_platform_server,
};
use nxtools_protocol::{ActivateWindowCommand, PlatformCommand, archive_read::*};
use serde_json::json;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

type ToolResult<T> = Result<T, &'static str>;

pub async fn run(root: &Path, scenario: &str, fixture: &mut Fixture) -> ToolResult<()> {
    let status = fixture.status.clone();
    let result = run_inner(root, scenario, fixture).await;
    if let Err(reason) = result {
        status.error(reason, None);
        status.request_stop("probe_failure");
        status.operation(json!({"status":"Failed","reason":reason}));
    }
    result
}
async fn run_inner(root: &Path, scenario: &str, fixture: &mut Fixture) -> ToolResult<()> {
    let status = fixture.status.clone();
    std::fs::create_dir(&fixture.root).map_err(|failure| {
        status.error("package_create_failure", failure.raw_os_error());
        "package_create_failure"
    })?;
    fixture.created = true;
    let workspace = exact_image(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .map_err(|_| "workspace_unavailable")?,
    );
    let worker = workspace.join("target/debug/nxtools-archive-worker.exe");
    let host_image = workspace.join(
        "csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe",
    );
    let helper = fixture.root.join("NxTools.ArchiveWorker.exe");
    std::fs::copy(&worker, &helper).map_err(|failure| {
        status.error("helper_copy_failure", failure.raw_os_error());
        "helper_copy_failure"
    })?;
    std::fs::write(fixture.root.join("capture-terminal"), b"")
        .map_err(|_| "terminal_marker_failure")?;
    let pressured = scenario == "product-pressure1gib" || scenario == "product-pressure-small";
    let cancel =
        scenario == "product-credit1gib" || scenario == "product-credit-small" || pressured;
    let source = root.join(if scenario.ends_with("small") {
        "credit.zip"
    } else if cancel {
        "1gib.zip"
    } else {
        "100k.zip"
    });
    let mut command = ArchiveReadCommand {
        version: 1,
        operation_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        generation: 1,
        action: ArchiveReadAction::Open,
        path: Some(source.to_str().ok_or("source_path_failure")?.to_owned()),
        codepage: 0,
        limits: Some(ArchiveReadLimits {
            ack_timeout_ms: 10000,
            ..Default::default()
        }),
        sequence: None,
    };
    let large = scenario == "product100k-v2";
    if large {
        command.version = 2;
        command.limits = Some(ArchiveReadLimits {
            max_entries: 100000,
            max_metadata_bytes: 128 << 20,
            max_retained_bytes: 64 << 20,
            max_transport_bytes: 64 << 20,
            ..Default::default()
        });
    }
    // Register exact images and original operation provenance BEFORE either host/worker launch.
    // The existing C# terminal capture exposes no worker PID. Python discovers only the exact
    // helper image with parent=host (never conhost); a missed worker is incomplete measurement.
    status.expect_images(&host_image, &helper, command.operation_id);
    status.bind_operation(command.operation_id);
    let pipe = format!("nxtools-archive-resource-{}", Uuid::new_v4());
    let (bridge, endpoint) = platform_bridge(32);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).map_err(|_| "client_init_failure")?;
    let pipe_copy = pipe.clone();
    let ep = endpoint.clone();
    fixture.server = Some(tokio::spawn(async move {
        let _ = run_platform_server(&pipe_copy, ep).await;
    }));
    // Let the listener exist before spawn: partial-init panic must still close original pipe.
    tokio::task::yield_now().await;
    if status.poll_stop() {
        return Ok(());
    }
    let host = Command::new(&host_image)
        .arg("--archive-product-host")
        .arg(&pipe)
        .arg(&fixture.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|failure| {
            status.error("host_spawn_failure", failure.raw_os_error());
            "host_spawn_failure"
        })?;
    let host_id = host.id();
    fixture.host = Some(host); // install ownership before status I/O/assertions
    status.identity(
        host_id,
        Some(std::process::id()),
        "isolated_csharp_host",
        &host_image,
        Some(command.operation_id),
    );
    status.host_parent(host_id);
    crate::emit(
        json!({"phase":"host_started","pid":host_id,"role":"isolated_csharp_host","package":fixture.root}),
    );
    let handshake_deadline = Instant::now() + Duration::from_secs(10);
    while !bridge.is_connected() {
        if status.poll_stop() {
            return Ok(());
        }
        if Instant::now() >= handshake_deadline {
            return Err("host_handshake_deadline");
        }
        tokio::time::sleep(POLL).await;
    }
    assert_ne!(
        scenario, "tool-guard-failure",
        "controlled TEST-tool failure after original pipe ownership"
    );
    if status.poll_stop() {
        return Ok(());
    }
    let original = bridge
        .connection_generation()
        .ok_or("original_connection_missing")?;
    if large {
        let mut default = command.clone();
        default.version = 1;
        default.limits = None;
        default.operation_id = Uuid::new_v4();
        let default_id = default.operation_id;
        fixture.sources.push((default_id, source.clone()));
        status.bind_operation(default_id);
        let limited = client.start(default).map_err(|_| "default_start_failure")?;
        let rejected = observe(
            &client,
            &bridge,
            &endpoint,
            &status,
            &fixture.root,
            &source,
            limited,
            false,
            false,
            false,
            Instant::now(),
            None,
        )
        .await?;
        if status.poll_stop() {
            return Ok(());
        }
        if rejected.terminal.code != Some(ArchiveReadError::ResourceLimit)
            || !rejected.terminal.cleanup_complete
            || rejected.terminal.entry_count != 0
        {
            return Err("default_refusal_failed");
        }
        status.fact("default_resource_limit", json!(true));
        status.bind_operation(command.operation_id);
    }
    let id = command.operation_id;
    // Install provenance before start: a panic immediately after dispatch still requires an
    // exact remote cleanup terminal. A start error is retained conservatively as unknown.
    fixture.sources.push((id, source.clone()));
    let clock = Instant::now();
    let operation = client
        .start(command.clone())
        .map_err(|_| "original_start_failure")?;
    status.operation(json!({"status":"Running","operation_id":id}));
    let mut manifest = if large {
        Some(Manifest::open(root)?)
    } else {
        None
    };
    let observation = observe(
        &client,
        &bridge,
        &endpoint,
        &status,
        &fixture.root,
        &source,
        operation,
        cancel,
        pressured,
        large,
        clock,
        manifest.as_mut(),
    )
    .await?;
    status.operation(json!({"status":if status.stopped() {"Stopped"} else {"ExpectedResult"},"operation_id":id,"terminal":observation.terminal}));
    if !status.stopped() {
        if large {
            assert_eq!(observation.terminal.status, ArchiveReadStatus::Succeeded);
            assert_eq!(observation.terminal.entry_count, 100000);
            assert!(observation.terminal.elapsed_ms <= 30000);
            let verified = manifest.as_ref().expect("independent manifest");
            crate::emit(
                json!({"phase":"manifest_verified","entries":verified.count,"batches":verified.batches,
                "snapshot_id":verified.snapshot,"serialized_received_bytes":verified.bytes,"all_fields_exact":true}),
            );
        } else {
            assert_eq!(
                observation.terminal.code,
                Some(if cancel {
                    ArchiveReadError::Cancelled
                } else {
                    ArchiveReadError::ResourceLimit
                })
            );
        }
        assert_eq!(bridge.connection_generation(), Some(original));
    }
    assert!(
        observation.terminal.cleanup_complete,
        "native remote cleanup required"
    );
    if let Err(failure) = std::fs::OpenOptions::new().write(true).open(&source) {
        status.error("source_release_unconfirmed", failure.raw_os_error());
        return Err("source_release_unconfirmed");
    }
    // Exact helper removal, only AFTER native terminal/source proof, never before cooperation.
    std::fs::remove_file(&helper).map_err(|failure| {
        status.error("helper_release_failure", failure.raw_os_error());
        "helper_release_failure"
    })?;
    status.fact("original_remote_terminal", json!(observation.terminal));
    status.fact("original_source_released", json!(true));
    status.fact("original_helper_released", json!(true));
    crate::emit(
        json!({"scenario":scenario,"operation_id":id,"host_pid":host_id,"full_terminal_ms":observation.elapsed_ms,"first_batch_ms":observation.first_batch_ms,"cancel_at_ms":observation.cancel_at_ms,"cancel_to_cleanup_upper_ms":observation.cancel_at_ms.map(|start|observation.elapsed_ms-start),"terminal":observation.terminal,"first_paint":"not_measured_no_ui"}),
    );
    // Resource stop is terminal for the whole probe. In particular NEVER re-admit a second
    // operation (nor invalid-escalation test) after stop, including a late post-terminal stop.
    if status.poll_stop() {
        return Ok(());
    }
    std::fs::copy(&worker, &helper).map_err(|failure| {
        status.error("helper_restore_failure", failure.raw_os_error());
        "helper_restore_failure"
    })?;
    if status.poll_stop() {
        return Ok(());
    }
    command.operation_id = Uuid::new_v4();
    command.path = Some(
        root.join("credit.zip")
            .to_str()
            .ok_or("source_path_failure")?
            .to_owned(),
    );
    status.bind_operation(command.operation_id);
    let reuse_source = root.join("credit.zip");
    fixture
        .sources
        .push((command.operation_id, reuse_source.clone()));
    if status.poll_stop() {
        fixture.sources.pop();
        return Ok(());
    }
    let next = client
        .start(command.clone())
        .map_err(|_| "reuse_start_failure")?;
    let reuse = observe(
        &client,
        &bridge,
        &endpoint,
        &status,
        &fixture.root,
        &reuse_source,
        next,
        false,
        false,
        true,
        Instant::now(),
        None,
    )
    .await?;
    if !status.stopped() {
        assert_eq!(reuse.terminal.status, ArchiveReadStatus::Succeeded);
    }
    assert!(reuse.terminal.cleanup_complete);
    if status.poll_stop() {
        return Ok(());
    }
    std::fs::remove_file(&helper).map_err(|failure| {
        status.error("reuse_helper_release_failure", failure.raw_os_error());
        "reuse_helper_release_failure"
    })?;
    command.operation_id = Uuid::new_v4();
    command.version = 1;
    command.limits = Some(ArchiveReadLimits {
        max_entries: 100000,
        ..Default::default()
    });
    if status.poll_stop() {
        return Ok(());
    }
    assert!(matches!(
        client.start(command),
        Err(ArchiveReadError::InvalidRequest)
    ));
    status.fact("same_router_reused", json!(true));
    Ok(())
}
struct Observation {
    terminal: ArchiveReadTerminal,
    elapsed_ms: f64,
    first_batch_ms: Option<f64>,
    cancel_at_ms: Option<f64>,
}
#[expect(
    clippy::too_many_arguments,
    reason = "one tool fixture original operation/stop/drain boundary"
)]
async fn observe(
    client: &ArchiveReadClient,
    bridge: &PlatformBridge,
    endpoint: &PlatformServerEndpoint,
    status: &ProbeStatus,
    package: &Path,
    source: &Path,
    operation: ArchiveReadOperation,
    planned_cancel: bool,
    pressured: bool,
    acknowledge: bool,
    clock: Instant,
    mut manifest: Option<&mut Manifest>,
) -> ToolResult<Observation> {
    let id = operation.operation_id();
    let terminal_path = package.join(format!("terminal-{id}.json"));
    let mut operation = Some(operation);
    let mut first_batch_ms = None;
    let mut cancel_at_ms = None;
    let mut poll_failed = false;
    let mut stop_dispatched = false;
    let operation_deadline = clock + Duration::from_secs(120);
    loop {
        if Instant::now() >= operation_deadline && !status.stopped() {
            status.request_stop("probe_deadline");
        }
        if status.poll_stop() && !stop_dispatched {
            stop_dispatched = true;
            status.phase("Cancelling");
            cancel_at_ms.get_or_insert(clock.elapsed().as_secs_f64() * 1000.);
            drop(operation.take()); // original coalesced cancellation route, full queue included
            status.phase("Draining");
        }
        match read_bounded(&terminal_path) {
            Ok(bytes) => {
                let terminal = serde_json::from_slice::<ArchiveReadTerminal>(&bytes)
                    .map_err(|_| "remote_terminal_invalid")?;
                if !terminal.cleanup_complete {
                    return Err("remote_cleanup_incomplete");
                }
                if manifest.is_some()
                    && let Some(active) = operation.as_ref()
                {
                    if active.terminal().is_none() {
                        let _ = tokio::time::timeout(POLL, client.poll()).await;
                        continue;
                    }
                    if active.terminal().as_ref() != Some(&terminal) {
                        return Err("owner_terminal_mismatch");
                    }
                }
                if terminal.status == ArchiveReadStatus::Succeeded
                    && !status.stopped()
                    && let Some(manifest) = manifest.as_mut()
                {
                    manifest.finish(&terminal)?;
                }
                return Ok(Observation {
                    terminal,
                    elapsed_ms: clock.elapsed().as_secs_f64() * 1000.,
                    first_batch_ms,
                    cancel_at_ms,
                });
            }
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {}
            Err(failure) => {
                status.error("remote_terminal_read_failure", failure.raw_os_error());
                return Err("remote_terminal_read_failure");
            }
        }
        // Reserve the final two seconds of the SAME ten-second cooperation budget for
        // original-pipe EOF and host await-using DrainAsync (including panic/init guard).
        if status.stopped() && status.cooperation_remaining() <= Duration::from_secs(2) {
            return Err("remote_drain_requires_pipe_eof");
        }
        tokio::select! {
            _ = tokio::time::sleep(POLL) => {}
            result = client.poll(), if !poll_failed => {
                if result.is_err() {
                    poll_failed = true; status.error("product_poll_failure", None); status.request_stop("probe_failure");
                    continue;
                }
                if manifest.is_some() && operation.as_ref().is_some_and(|active| active.terminal().is_some_and(|terminal| terminal.status != ArchiveReadStatus::Succeeded)) {
                    // A local disconnect/error is not a remote cleanup proof. Fail promptly;
                    // the original guard still closes/drains the owned pipe and retains evidence.
                    return Err("product_terminal_before_complete_manifest");
                }
                if let Some(active) = operation.as_ref()
                    && let Some(batch) = active.take_batch()
                {
                        first_batch_ms.get_or_insert(clock.elapsed().as_secs_f64()*1000.);
                        if let Some(manifest) = manifest.as_mut() { manifest.batch(&batch)?; }
                        if acknowledge { active.acknowledge(batch.sequence).map_err(|_| "reuse_ack_failure")?; }
                        else if planned_cancel && cancel_at_ms.is_none() {
                            assert!(std::fs::OpenOptions::new().write(true).open(source).is_err(), "original source protected");
                            cancel_at_ms = Some(clock.elapsed().as_secs_f64()*1000.);
                            if pressured {
                                while bridge.notification_receiver().try_recv().is_ok() {}
                                let pause = endpoint.pause_commands_for_contract_test().map_err(|_| "queue_barrier_failure")?;
                                for _ in 0..32 { bridge.send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand{window_handle:1})).map_err(|_| "queue_fill_failure")?; }
                                assert_eq!(bridge.send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand{window_handle:1})), Err(PlatformBridgeError::Busy));
                                drop(operation.take()); drop(pause);
                            } else {
                                active.cancel().map_err(|_| "original_cancel_failure")?;
                                assert!(active.terminal().is_none_or(|terminal| !terminal.cleanup_complete), "receipt is not native cleanup");
                            }
                        } else { assert!(planned_cancel, "100k must not succeed truncated"); }
                    }
            }
        }
    }
}

struct Manifest {
    lines: std::io::Lines<BufReader<std::fs::File>>,
    count: u64,
    batches: u64,
    bytes: u64,
    snapshot: Option<Uuid>,
}
impl Manifest {
    fn open(root: &Path) -> ToolResult<Self> {
        Ok(Self {
            lines: BufReader::new(
                std::fs::File::open(root.join("entries.jsonl")).map_err(|_| "manifest_open")?,
            )
            .lines(),
            count: 0,
            batches: 0,
            bytes: 0,
            snapshot: None,
        })
    }
    fn batch(&mut self, batch: &ArchiveReadBatch) -> ToolResult<()> {
        if batch.sequence != self.batches
            || self.snapshot.is_some_and(|id| id != batch.snapshot_id)
            || batch.format != "Zip"
        {
            return Err("manifest_identity");
        }
        self.snapshot = Some(batch.snapshot_id);
        self.bytes += batch.validate_for(2).map_err(|_| "manifest_batch_bound")? as u64;
        for entry in &batch.entries {
            let row: serde_json::Value = serde_json::from_str(
                &self
                    .lines
                    .next()
                    .ok_or("manifest_short")?
                    .map_err(|_| "manifest_read")?,
            )
            .map_err(|_| "manifest_json")?;
            if row["ordinal"] != entry.ordinal
                || row["name"] != entry.name
                || row["kind"] != entry.kind
                || row["size"].as_u64() != entry.size
                || row["packed_size"].as_u64() != entry.packed_size
                || entry.ordinal != self.count
                || entry.encrypted != Some(false)
                || entry.provenance
                    != if entry.name.is_ascii() {
                        "UncertainCp437"
                    } else {
                        "Utf8Flag"
                    }
                || entry.codepage != Some(if entry.name.is_ascii() { 437 } else { 65001 })
            {
                return Err("manifest_mismatch");
            }
            self.count += 1;
        }
        self.batches += 1;
        Ok(())
    }
    fn finish(&mut self, terminal: &ArchiveReadTerminal) -> ToolResult<()> {
        if self.count != 100000
            || self.lines.next().is_some()
            || terminal.entry_count != self.count
            || terminal.batch_count != self.batches
            || terminal
                .transfer
                .as_ref()
                .is_none_or(|v| v.private_bytes > 64 << 20 || v.max_inflight_batches != 1)
        {
            return Err("manifest_incomplete");
        }
        Ok(())
    }
}
