#![cfg(windows)]
use nxtools_archive::product::{ArchiveReadClient, ArchiveReadOperation};
use nxtools_ipc::{
    PlatformBridgeError, PlatformServerEndpoint, platform_bridge, run_platform_server,
};
use nxtools_protocol::{ActivateWindowCommand, PlatformCommand, archive_read::*};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

// No product runtime/installed host is launched. Only the explicitly named test apphost.
struct Fixture {
    root: PathBuf,
    child: Option<Child>,
    server: Option<tokio::task::JoinHandle<()>>,
    endpoint: Option<PlatformServerEndpoint>,
    pipe: Option<String>,
    retain_inputs: bool,
}
impl Fixture {
    async fn new() -> (Self, ArchiveReadClient, Arc<nxtools_ipc::PlatformBridge>) {
        let scratch = std::env::var_os("PI_SCRATCH_DIR")
            .map(PathBuf::from)
            .expect("session scratch");
        let root = scratch.join(format!("archive-product-chain-{}", Uuid::new_v4()));
        let mut fixture = Self {
            root: root.clone(),
            child: None,
            server: None,
            endpoint: None,
            pipe: None,
            retain_inputs: false,
        };
        std::fs::create_dir_all(root.join("package")).expect("owned package");
        std::fs::copy(
            env!("CARGO_BIN_EXE_nxtools-archive-worker"),
            root.join("package/NxTools.ArchiveWorker.exe"),
        )
        .expect("actual Rust worker in isolated package");
        let python=Command::new("python").args(["-c", "import sys,zipfile\nwith zipfile.ZipFile(sys.argv[1],'w',compression=zipfile.ZIP_STORED) as z:\n z.writestr('alpha.txt',b'alpha\\n')\n z.writestr('目录/',b'')\n z.writestr('目录/你好.txt',b'hello\\n')",root.join("literal.zip").to_str().expect("fixture path")]).output().expect("independent Python ZIP fixture");
        assert!(python.status.success(), "fixture generation failed");
        let pipe = format!("nxtools-archive-product-{}", Uuid::new_v4());
        fixture.pipe = Some(pipe.clone());
        let (bridge, endpoint) = platform_bridge(32);
        let bridge = Arc::new(bridge);
        fixture.endpoint = Some(endpoint.clone());
        let client = ArchiveReadClient::new(bridge.clone()).expect("real product read owner");
        let pipe_server = pipe.clone();
        let server = tokio::spawn(async move {
            let _ = run_platform_server(&pipe_server, endpoint).await;
        });
        fixture.server = Some(server);
        tokio::task::yield_now().await;
        let apphost=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        assert!(apphost.is_file(), "build isolated C# test apphost first");
        let child = Command::new(apphost)
            .arg("--archive-product-host")
            .arg(&pipe)
            .arg(root.join("package"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("owned C# product seam host");
        fixture.child = Some(child);
        tokio::time::timeout(Duration::from_secs(5), async {
            while !bridge.is_connected() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("actual typed IPC handshake");
        (fixture, client, bridge)
    }
    fn request(&self, action: ArchiveReadAction) -> ArchiveReadCommand {
        ArchiveReadCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            action,
            path: Some(
                self.root
                    .join("literal.zip")
                    .to_str()
                    .expect("path")
                    .to_owned(),
            ),
            codepage: 0,
            limits: None,
            sequence: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
        }
        if let Some(mut child) = self.child.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        if self.retain_inputs {
            return; // Long-path source/evidence is intentionally retained, never source-deleted.
        }
        // Windows can briefly retain an exited Job's image mapping; test cleanup is bounded.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            match std::fs::remove_dir_all(&self.root) {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(_) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                Err(error) => {
                    if std::thread::panicking() {
                        eprintln!("owned archive fixture cleanup failed: {:?}", error.kind());
                    } else {
                        panic!("owned archive fixture cleanup failed: {:?}", error.kind());
                    }
                    break;
                }
            }
        }
    }
}
async fn consume(
    client: &ArchiveReadClient,
    operation: &ArchiveReadOperation,
) -> (Vec<ArchiveReadBatch>, ArchiveReadTerminal) {
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut batches = Vec::new();
        loop {
            client.poll().await.expect("product owner event processing");
            if let Some(batch) = operation.take_batch() {
                let seq = batch.sequence;
                batches.push(batch);
                operation
                    .acknowledge(seq)
                    .expect("exact retained metadata acknowledgement");
            }
            if let Some(terminal) = operation.terminal() {
                return (batches, terminal);
            }
        }
    })
    .await
    .expect("bounded product terminal")
}
#[tokio::test(flavor = "current_thread")]
async fn product_open_uses_typed_pipe_original_job_and_actual_worker() {
    let (fixture, client, _bridge) = Fixture::new().await;
    let operation = client
        .start(fixture.request(ArchiveReadAction::Open))
        .expect("product open admission");
    let (batches, terminal) = consume(&client, &operation).await;
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "real product open terminal: {:?}",
        terminal.code
    );
    assert!(terminal.cleanup_complete);
    assert_eq!((terminal.entry_count, terminal.batch_count), (3, 2));
    let entries = batches
        .iter()
        .flat_map(|batch| &batch.entries)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["alpha.txt", "目录/", "目录/你好.txt"]
    );
    assert_eq!(
        entries.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
        ["File", "Directory", "File"]
    );
    assert_eq!(
        entries.iter().map(|e| e.size).collect::<Vec<_>>(),
        [Some(6), Some(0), Some(6)]
    );
    assert_eq!(
        entries.iter().map(|e| e.ordinal).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        (entries[1].provenance.as_str(), entries[1].codepage),
        ("Utf8Flag", Some(65001))
    );
    assert_eq!(batches[0].snapshot_id, batches[1].snapshot_id);
    assert!(!batches[0].snapshot_id.is_nil());
    assert_eq!(batches[0].format, "Zip");
    assert_eq!(
        std::fs::read_dir(&fixture.root)
            .expect("owned root")
            .count(),
        2,
        "read-only operation creates no output"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn product_auto_encoding_reports_actual_candidate_and_uncertain_fallback() {
    let (mut fixture, client, _bridge) = Fixture::new().await;
    fixture.retain_inputs = true;
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zip_encoding.py");
    assert!(
        Command::new("python")
            .arg(script)
            .arg(&fixture.root)
            .status()
            .expect("independent raw ZIPs")
            .success()
    );
    for (file, name, provenance, cp) in [
        ("auto-gbk.zip", "中文文件资料.txt", "Heuristic", 936),
        ("ambiguous.zip", "╓╨╬─.txt", "UncertainCp437", 437),
    ] {
        let mut request = fixture.request(ArchiveReadAction::Open);
        request.path = Some(
            fixture
                .root
                .join(file)
                .to_str()
                .expect("fixture path")
                .to_owned(),
        );
        let operation = client.start(request).expect("original product admission");
        let (batches, terminal) = consume(&client, &operation).await;
        assert_eq!(
            terminal.status,
            ArchiveReadStatus::Succeeded,
            "{:?}",
            terminal.code
        );
        assert!(terminal.cleanup_complete);
        assert_eq!((terminal.entry_count, terminal.batch_count), (1, 1));
        let entry = &batches[0].entries[0];
        assert_eq!(
            (
                entry.name.as_str(),
                entry.provenance.as_str(),
                entry.codepage
            ),
            (name, provenance, Some(cp))
        );
        assert_eq!(entry.ordinal, 0);
        assert_eq!(entry.size, Some(29));
        assert!(
            entry
                .warning_codes
                .iter()
                .any(|value| value == "UncertainEncoding")
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn product_test_counts_bytes_and_reports_crc_corruption_without_output() {
    let (fixture, client, _bridge) = Fixture::new().await;
    let operation = client
        .start(fixture.request(ArchiveReadAction::Test))
        .expect("test admission");
    let (batches, terminal) = consume(&client, &operation).await;
    assert!(batches.is_empty());
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "{:?}",
        terminal.code
    );
    assert_eq!((terminal.files_verified, terminal.bytes_verified), (2, 12));
    assert!(terminal.cleanup_complete);
    let source = fixture.root.join("literal.zip");
    let mut bytes = std::fs::read(&source).expect("owned ZIP");
    let position = bytes
        .windows(6)
        .position(|v| v == b"alpha\n")
        .expect("independent stored payload");
    bytes[position] ^= 1;
    std::fs::write(&source, &bytes).expect("lease released before terminal");
    let damaged = client
        .start(fixture.request(ArchiveReadAction::Test))
        .expect("damaged test admission");
    let (_, terminal) = consume(&client, &damaged).await;
    assert_eq!(terminal.code, Some(ArchiveReadError::CorruptArchive));
    assert_eq!(terminal.status, ArchiveReadStatus::Failed);
    assert!(terminal.cleanup_complete);
    assert_eq!(
        std::fs::read_dir(&fixture.root)
            .expect("owned root")
            .count(),
        2
    );
}

#[tokio::test(flavor = "current_thread")]
async fn paused_business_consumer_times_out_and_cancel_wakes_original_worker() {
    let (fixture, client, _bridge) = Fixture::new().await;
    for cancel in [false, true] {
        let mut command = fixture.request(ArchiveReadAction::Open);
        command.limits = Some(ArchiveReadLimits {
            ack_timeout_ms: 500,
            ..Default::default()
        });
        let operation = client.start(command).expect("open");
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                client.poll().await.expect("event");
                if let Some(batch) = operation.take_batch() {
                    assert_eq!(batch.sequence, 0);
                    break;
                }
            }
        })
        .await
        .expect("first batch");
        assert!(operation.terminal().is_none());
        assert_eq!(
            client.queued_events(),
            0,
            "no next metadata without business credit"
        );
        // The source is still protected while metadata credit is outstanding.
        let writable = std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"));
        assert!(writable.is_err());
        if cancel {
            operation.cancel().expect("cancel");
            operation.cancel().expect("duplicate cancel");
        }
        let (_, terminal) = consume(&client, &operation).await;
        assert_eq!(
            terminal.code,
            Some(if cancel {
                ArchiveReadError::Cancelled
            } else {
                ArchiveReadError::ConsumerTooSlow
            })
        );
        assert!(terminal.cleanup_complete);
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(fixture.root.join("literal.zip"))
                .is_ok()
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn product_metadata_retained_budget_refuses_and_releases_native_lease() {
    let (fixture, client, _bridge) = Fixture::new().await;
    let mut command = fixture.request(ArchiveReadAction::Open);
    command.limits = Some(ArchiveReadLimits {
        max_retained_bytes: 1,
        ..Default::default()
    });
    let operation = client.start(command).expect("bounded admission");
    let (batches, terminal) = consume(&client, &operation).await;
    assert!(batches.is_empty());
    assert_eq!(terminal.code, Some(ArchiveReadError::MetadataBudget));
    assert!(terminal.cleanup_complete);
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_ok()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn actual_host_disconnect_retires_original_read_and_releases_lease() {
    let (mut fixture, client, _bridge) = Fixture::new().await;
    let operation = client
        .start(fixture.request(ArchiveReadAction::Open))
        .expect("open");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            client.poll().await.expect("event");
            if operation.take_batch().is_some() {
                break;
            }
        }
    })
    .await
    .expect("first batch owns lease");
    let child = fixture.child.as_mut().expect("owned host");
    child.kill().expect("terminate only owned isolated host");
    child.wait().expect("reap owned host");
    tokio::time::timeout(Duration::from_secs(5), async {
        while operation.terminal().is_none() {
            client.poll().await.expect("actual pipe EOF");
        }
    })
    .await
    .expect("disconnect terminal");
    assert_eq!(
        operation.terminal().expect("terminal").code,
        Some(ArchiveReadError::Disconnected)
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_err()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("owned Job death closes native source lease");
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_product_consumer_cancels_original_without_output_or_replacement() {
    let (fixture, client, _bridge) = Fixture::new().await;
    let operation = client
        .start(fixture.request(ArchiveReadAction::Open))
        .expect("open");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            client.poll().await.expect("event");
            if operation.take_batch().is_some() {
                break;
            }
        }
    })
    .await
    .expect("first batch");
    drop(client);
    assert_eq!(
        operation.terminal().expect("retired consumer").code,
        Some(ArchiveReadError::Cancelled)
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_err()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancel reaches actual runner and closes lease");
    assert_eq!(
        std::fs::read_dir(&fixture.root)
            .expect("owned root")
            .count(),
        2
    );
}

#[tokio::test(flavor = "current_thread")]
async fn unavailable_source_remains_typed_with_actual_native_error_after_cleanup() {
    let (fixture, client, _bridge) = Fixture::new().await;
    let mut command = fixture.request(ArchiveReadAction::Open);
    command.path = Some(
        fixture
            .root
            .join("missing-owned.zip")
            .to_str()
            .expect("owned path")
            .to_owned(),
    );
    let operation = client
        .start(command)
        .expect("source admission is asynchronous");
    let (_, terminal) = consume(&client, &operation).await;
    assert_eq!(terminal.code, Some(ArchiveReadError::SourceUnavailable));
    assert!(
        matches!(terminal.native_error_code, Some(2 | 3)),
        "native source boundary code"
    );
    assert!(terminal.cleanup_complete);
}

async fn pressured_consumer_drop(drop_client: bool, version: u32) {
    let (fixture, owner, bridge) = Fixture::new().await;
    let mut client = Some(owner);
    let mut command = fixture.request(ArchiveReadAction::Open);
    if version == 2 {
        replace_zip(&fixture, 600, false);
        command.version = 2;
    }
    command.limits = Some(ArchiveReadLimits {
        ack_timeout_ms: 10000,
        max_entries: if version == 2 { 5000 } else { 4096 },
        max_metadata_bytes: if version == 2 { 128 << 20 } else { 4 << 20 },
        max_retained_bytes: if version == 2 { 64 << 20 } else { 4 << 20 },
        max_transport_bytes: if version == 2 { 64 << 20 } else { 8 << 20 },
        ..Default::default()
    });
    let id = command.operation_id;
    let operation = client
        .as_ref()
        .expect("live client")
        .start(command)
        .expect("actual read");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            client
                .as_ref()
                .expect("live client")
                .poll()
                .await
                .expect("original event");
            if operation.take_batch().is_some() {
                break;
            }
        }
    })
    .await
    .expect("original worker/source lease and business-credit barrier");
    let source = fixture.root.join("literal.zip");
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .is_err(),
        "original source protected before Drop"
    );
    std::fs::write(fixture.root.join("package/capture-terminal"), b"")
        .expect("owned terminal observation");
    while bridge.notification_receiver().try_recv().is_ok() {}
    let original = bridge.connection_generation().expect("original connection");
    let paused = fixture
        .endpoint
        .as_ref()
        .expect("endpoint")
        .pause_commands_for_contract_test()
        .expect("pause actual production command dequeue");
    for _ in 0..32 {
        bridge
            .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                window_handle: 1,
            }))
            .expect("controlled filler occupies one existing slot");
    }
    assert_eq!(
        bridge.send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle: 1
        })),
        Err(PlatformBridgeError::Busy),
        "ordinary shared command queue is deterministically full"
    );
    if drop_client {
        drop(client.take());
        assert!(
            !operation
                .terminal()
                .expect("local cancellation")
                .cleanup_complete
        );
        drop(operation);
    } else {
        drop(operation);
        // Keep client alive but do not poll; operation Drop must suffice by itself.
    }
    drop(paused);
    let terminal_path = fixture.root.join(format!("package/terminal-{id}.json"));
    let terminal = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(bytes) = std::fs::read(&terminal_path) {
                break serde_json::from_slice::<ArchiveReadTerminal>(&bytes)
                    .expect("actual production cleanup terminal");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("retained cancel reaches original session/runner/Job before credit timeout");
    assert_eq!(
        terminal.code,
        Some(ArchiveReadError::Cancelled),
        "not timeout/EOF cleanup"
    );
    assert!(
        terminal.cleanup_complete,
        "actual runner process/pumps/Job/resource cleanup"
    );
    assert_eq!(
        bridge.connection_generation(),
        Some(original),
        "not local-only disconnect or replacement"
    );
    assert!(
        std::fs::OpenOptions::new().write(true).open(source).is_ok(),
        "actual original source lease released"
    );
    let helper = fixture.root.join("package/NxTools.ArchiveWorker.exe");
    std::fs::remove_file(helper).expect("original helper image lease released after Job cleanup");
}

#[tokio::test(flavor = "current_thread")]
async fn pressured_operation_drop_releases_actual_original_worker_job_and_source() {
    pressured_consumer_drop(false, 1).await;
}
#[tokio::test(flavor = "current_thread")]
async fn pressured_client_drop_releases_actual_original_worker_job_and_source() {
    pressured_consumer_drop(true, 1).await;
}

#[test]
fn fixture_guard_cleans_helper_when_initialization_panics() {
    let root = PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("scratch"))
        .join(format!("archive-product-chain-{}", Uuid::new_v4()));
    let owned = root.clone();
    let failed = std::panic::catch_unwind(move || {
        let _guard = Fixture {
            root: owned.clone(),
            child: None,
            server: None,
            endpoint: None,
            pipe: None,
            retain_inputs: false,
        };
        std::fs::create_dir_all(owned.join("package")).expect("package");
        std::fs::write(
            owned.join("package/NxTools.ArchiveWorker.exe"),
            b"owned fixture residue",
        )
        .expect("owned helper");
        panic!("controlled initialization failure");
    });
    assert!(failed.is_err());
    assert!(
        !root.exists(),
        "guard established before any fallible initialization"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn ordinary_receipt_saturation_before_drop_retires_actual_pipe_and_original_job() {
    let (mut fixture, client, bridge) = Fixture::new().await;
    let mut command = fixture.request(ArchiveReadAction::Open);
    command.limits = Some(ArchiveReadLimits {
        ack_timeout_ms: 10000,
        ..Default::default()
    });
    let operation = client.start(command).expect("actual original read");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            client.poll().await.expect("credit barrier");
            if operation.take_batch().is_some() {
                break;
            }
        }
    })
    .await
    .expect("original source/Job lease established");
    let source = fixture.root.join("literal.zip");
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .is_err()
    );
    let endpoint = fixture.endpoint.as_ref().expect("original endpoint");
    while bridge.notification_receiver().try_recv().is_ok() {}
    for _ in 0..32 {
        endpoint
            .send_notification_for_contract_test(nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: Uuid::new_v4(),
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            })
            .expect("fill unrelated notification queue deterministically");
    }
    let filler = bridge
        .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle: 1,
        }))
        .expect("ordinary receipt reaches saturated notifications before Drop");
    tokio::time::timeout(Duration::from_secs(5), async {
        while endpoint.command_pending_for_contract_test(filler) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("ordinary result crossed production complete-command boundary");
    drop(client);
    assert!(!operation.terminal().expect("local only").cleanup_complete);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let exited = fixture
                .child
                .as_mut()
                .expect("owned host")
                .try_wait()
                .expect("host state")
                .is_some();
            if exited
                && std::fs::OpenOptions::new()
                    .write(true)
                    .open(&source)
                    .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect(
        "real original pipe EOF must cancel and drain original Job/source before credit timeout",
    );
    assert!(
        !bridge.is_connected(),
        "original connection is really retired, not local-only Cancelled"
    );
    std::fs::remove_file(fixture.root.join("package/NxTools.ArchiveWorker.exe"))
        .expect("original Job/helper image cleanup confirmed");
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_finite_v2_uses_actual_product_chain_without_raising_default_limits() {
    let (fixture, client, bridge) = Fixture::new().await;
    let connection = bridge.connection_generation();
    let mut conservative = fixture.request(ArchiveReadAction::Open);
    conservative.limits = Some(ArchiveReadLimits {
        max_entries: 2,
        ..Default::default()
    });
    let limited = client.start(conservative).expect("ordinary finite request");
    let (batches, terminal) = consume(&client, &limited).await;
    assert!(batches.is_empty());
    assert_eq!(terminal.code, Some(ArchiveReadError::ResourceLimit));
    assert!(terminal.cleanup_complete);

    let mut command = fixture.request(ArchiveReadAction::Open);
    command.version = 2;
    command.limits = Some(ArchiveReadLimits {
        max_entries: 5000,
        max_metadata_bytes: 128 << 20,
        max_retained_bytes: 64 << 20,
        ..Default::default()
    });
    let operation = client.start(command).expect("explicit finite v2 admission");
    let (batches, terminal) = consume(&client, &operation).await;
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "{:?}",
        terminal.code
    );
    assert!(terminal.cleanup_complete);
    assert_eq!((terminal.entry_count, terminal.batch_count), (3, 1));
    assert_eq!(batches.len(), 1);
    assert_eq!(
        batches[0]
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha.txt", "目录/", "目录/你好.txt"]
    );
    assert_eq!(bridge.connection_generation(), connection);
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_ok()
    );
    std::fs::remove_file(fixture.root.join("package/NxTools.ArchiveWorker.exe"))
        .expect("actual cleanup releases original helper lease");
}

fn large_request(fixture: &Fixture) -> ArchiveReadCommand {
    let mut command = fixture.request(ArchiveReadAction::Open);
    command.version = 2;
    command.limits = Some(ArchiveReadLimits {
        max_entries: 5000,
        max_metadata_bytes: 128 << 20,
        max_retained_bytes: 64 << 20,
        max_transport_bytes: 64 << 20,
        ..Default::default()
    });
    command
}
fn replace_zip(fixture: &Fixture, count: usize, long_names: bool) {
    let result = Command::new("python").args(["-c",
        "import sys,zipfile\nwith zipfile.ZipFile(sys.argv[1],'w',compression=zipfile.ZIP_STORED) as z:\n for i in range(int(sys.argv[2])): z.writestr(('汉'*900 if sys.argv[3]=='long' else 'literal-')+str(i)+'.txt',b'abc')",
        fixture.root.join("literal.zip").to_str().expect("path"), &count.to_string(), if long_names {"long"} else {"short"}])
        .output().expect("independent fixture");
    assert!(result.status.success());
}

#[tokio::test(flavor = "current_thread")]
async fn v2_multiple_batches_exact_member_limit_and_one_over_are_not_truncated() {
    let (fixture, client, bridge) = Fixture::new().await;
    replace_zip(&fixture, 600, false);
    let connection = bridge.connection_generation();
    let mut command = large_request(&fixture);
    command.limits.as_mut().expect("limits").max_entries = 600;
    let operation = client.start(command.clone()).expect("exact finite limit");
    let (batches, terminal) = consume(&client, &operation).await;
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "{:?}",
        terminal.code
    );
    assert_eq!((terminal.entry_count, terminal.batch_count), (600, 3));
    assert_eq!(
        batches
            .iter()
            .map(|batch| batch.entries.len())
            .collect::<Vec<_>>(),
        [256, 256, 88]
    );
    let snapshot = batches[0].snapshot_id;
    let mut ordinal = 0;
    for batch in &batches {
        assert_eq!(batch.snapshot_id, snapshot);
        assert!(batch.validate_for(2).expect("bounded batch") <= 256 << 10);
        for entry in &batch.entries {
            assert_eq!(entry.ordinal, ordinal);
            assert_eq!(entry.name, format!("literal-{ordinal}.txt"));
            ordinal += 1;
        }
    }
    command.operation_id = Uuid::new_v4();
    command.limits.as_mut().expect("limits").max_entries = 599;
    let refused = client.start(command).expect("finite request admission");
    let (batches, terminal) = consume(&client, &refused).await;
    assert!(batches.is_empty());
    assert_eq!(terminal.code, Some(ArchiveReadError::ResourceLimit));
    assert!(terminal.cleanup_complete);
    assert_eq!(bridge.connection_generation(), connection);
}

#[tokio::test(flavor = "current_thread")]
async fn v2_byte_bounded_unicode_batches_keep_every_member() {
    let (fixture, client, _) = Fixture::new().await;
    replace_zip(&fixture, 120, true);
    let operation = client.start(large_request(&fixture)).expect("explicit v2");
    let (batches, terminal) = consume(&client, &operation).await;
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "{:?}",
        terminal.code
    );
    assert_eq!(terminal.entry_count, 120);
    assert!(
        batches.len() >= 3,
        "serialized byte bound must split before entry count"
    );
    assert_eq!(
        batches
            .iter()
            .map(|batch| batch.entries.len())
            .sum::<usize>(),
        120
    );
    assert!(terminal.cleanup_complete);
}

#[tokio::test(flavor = "current_thread")]
async fn v2_cumulative_transport_and_retention_refuse_without_successful_truncation() {
    let (fixture, client, _) = Fixture::new().await;
    replace_zip(&fixture, 600, false);
    for transport in [false, true] {
        let mut command = large_request(&fixture);
        let limits = command.limits.as_mut().expect("explicit limits");
        if transport {
            limits.max_transport_bytes = 60000;
        } else {
            limits.max_retained_bytes = 60000;
        }
        let operation = client.start(command).expect("finite quota");
        let (batches, terminal) = consume(&client, &operation).await;
        assert_eq!(terminal.code, Some(ArchiveReadError::MetadataBudget));
        assert_ne!(terminal.status, ArchiveReadStatus::Succeeded);
        assert!(terminal.cleanup_complete);
        assert!(
            batches
                .iter()
                .map(|batch| batch.entries.len())
                .sum::<usize>()
                < 600
        );
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(fixture.root.join("literal.zip"))
                .is_ok()
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn v2_multibatch_credit_timeout_cancel_and_drop_preserve_native_owners() {
    let (fixture, client, bridge) = Fixture::new().await;
    replace_zip(&fixture, 600, false);
    std::fs::write(fixture.root.join("package/capture-terminal"), b"")
        .expect("owned terminal observation");
    let connection = bridge.connection_generation();
    for mode in ["timeout", "cancel", "drop"] {
        let mut command = large_request(&fixture);
        command.limits.as_mut().expect("limits").ack_timeout_ms = 500;
        let id = command.operation_id;
        let operation = client.start(command).expect("v2 admission");
        for sequence in 0..2 {
            loop {
                client.poll().await.expect("event");
                if let Some(batch) = operation.take_batch() {
                    assert_eq!(batch.sequence, sequence);
                    assert_eq!(batch.entries.len(), 256);
                    assert_eq!(
                        operation.acknowledge(sequence + 1),
                        Err(ArchiveReadError::ProtocolViolation)
                    );
                    if sequence == 0 {
                        operation.acknowledge(sequence).expect("exact credit");
                        assert_eq!(
                            operation.acknowledge(sequence),
                            Err(ArchiveReadError::ProtocolViolation)
                        );
                    }
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(operation.terminal().is_none());
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(fixture.root.join("literal.zip"))
                .is_err()
        );
        assert!(
            std::fs::remove_file(fixture.root.join("package/NxTools.ArchiveWorker.exe")).is_err()
        );
        if mode == "drop" {
            drop(operation);
        } else {
            if mode == "cancel" {
                operation.cancel().expect("cancel");
                operation.cancel().expect("coalesce");
            }
            let (_, terminal) = consume(&client, &operation).await;
            assert_eq!(
                terminal.code,
                Some(if mode == "timeout" {
                    ArchiveReadError::ConsumerTooSlow
                } else {
                    ArchiveReadError::Cancelled
                })
            );
            assert!(terminal.cleanup_complete);
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while !fixture
                .root
                .join(format!("package/terminal-{id}.json"))
                .is_file()
            {
                let _ = tokio::time::timeout(Duration::from_millis(20), client.poll()).await;
            }
        })
        .await
        .expect("original remote cleanup");
        let terminal: ArchiveReadTerminal = serde_json::from_slice(
            &std::fs::read(fixture.root.join(format!("package/terminal-{id}.json")))
                .expect("capture"),
        )
        .expect("terminal");
        assert!(terminal.cleanup_complete);
        assert_ne!(terminal.status, ArchiveReadStatus::Succeeded);
        assert_eq!(bridge.connection_generation(), connection);
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(fixture.root.join("literal.zip"))
                .is_ok()
        );
    }
    let operation = client
        .start(large_request(&fixture))
        .expect("same connection reusable");
    assert_eq!(
        consume(&client, &operation).await.1.status,
        ArchiveReadStatus::Succeeded
    );
}

#[tokio::test(flavor = "current_thread")]
async fn v2_cumulative_retained_exact_byte_boundaries_and_one_less_are_enforced() {
    let (fixture, client, _) = Fixture::new().await;
    let original = client.start(large_request(&fixture)).expect("v2");
    let (_, terminal) = consume(&client, &original).await;
    let stats = terminal.transfer.expect("v2 actual counts");
    for delta in [0, 1] {
        let mut command = large_request(&fixture);
        let limits = command.limits.as_mut().expect("limits");
        limits.max_retained_bytes = stats.received_bytes - delta;
        let operation = client.start(command).expect("explicit finite quota");
        let (_, terminal) = consume(&client, &operation).await;
        assert!(terminal.cleanup_complete);
        if delta == 0 {
            assert_eq!(
                terminal.status,
                ArchiveReadStatus::Succeeded,
                "{:?}",
                terminal.code
            );
        } else {
            assert_eq!(terminal.code, Some(ArchiveReadError::MetadataBudget));
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn v2_disconnect_reconnect_rejects_old_generation_and_reuses_cleaned_session() {
    let (mut fixture, client, bridge) = Fixture::new().await;
    replace_zip(&fixture, 600, false);
    let command = large_request(&fixture);
    let previous = bridge.connection_generation().expect("connection");
    let operation = client.start(command.clone()).expect("v2");
    loop {
        client.poll().await.expect("event");
        if operation.take_batch().is_some() {
            break;
        }
    }
    fixture
        .child
        .as_mut()
        .expect("owned host")
        .kill()
        .expect("owned disconnect");
    fixture
        .child
        .as_mut()
        .expect("owned host")
        .wait()
        .expect("reap");
    tokio::time::timeout(Duration::from_secs(5), async {
        while operation.terminal().is_none() {
            client.poll().await.expect("EOF");
        }
        while std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_err()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("original pipe/Job/source release");
    let apphost=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
    // The existing server accepts a replacement client on the SAME endpoint/generation owner.
    let pipe = fixture.pipe.as_ref().expect("original pipe");
    fixture.child = Some(
        Command::new(apphost)
            .arg("--archive-product-host")
            .arg(pipe)
            .arg(fixture.root.join("package"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("replacement isolated host"),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while bridge
            .connection_generation()
            .is_none_or(|value| value == previous)
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("new connection");
    let mut old = command;
    old.action = ArchiveReadAction::Cancel;
    old.path = None;
    old.limits = None;
    assert_eq!(
        bridge.send_archive_read(old, Uuid::new_v4(), previous),
        Err(PlatformBridgeError::Disconnected)
    );
    let next = client
        .start(large_request(&fixture))
        .expect("new generation");
    assert_eq!(
        consume(&client, &next).await.1.status,
        ArchiveReadStatus::Succeeded
    );
}

#[tokio::test(flavor = "current_thread")]
async fn v2_full_command_queue_drop_preserves_original_cancel_and_cleanup() {
    pressured_consumer_drop(false, 2).await;
    pressured_consumer_drop(true, 2).await;
}

#[tokio::test(flavor = "current_thread")]
async fn v2_del_escaped_member_names_split_before_business_batch_ceiling() {
    let (fixture, client, _) = Fixture::new().await;
    let generated=Command::new("python").args(["-c","import sys,zipfile\nwith zipfile.ZipFile(sys.argv[1],'w') as z:\n for i in range(100): z.writestr(chr(127)*1000+str(i),b'')",fixture.root.join("literal.zip").to_str().expect("path")]).output().expect("CPython");
    assert!(generated.status.success());
    let operation = client.start(large_request(&fixture)).expect("v2");
    let (batches, terminal) = consume(&client, &operation).await;
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "{:?}",
        terminal.code
    );
    assert_eq!(terminal.entry_count, 100);
    assert!(batches.len() >= 3);
    assert!(terminal.cleanup_complete);
}

#[tokio::test(flavor = "current_thread")]
async fn v2_above_default_directory_fully_crosses_product_pipe() {
    let (fixture, client, _) = Fixture::new().await;
    replace_zip(&fixture, 5000, false);
    let operation = client
        .start(large_request(&fixture))
        .expect("explicit finite request");
    let (batches, terminal) = consume(&client, &operation).await;
    assert_eq!(
        terminal.status,
        ArchiveReadStatus::Succeeded,
        "{:?}",
        terminal.code
    );
    assert_eq!(terminal.entry_count, 5000);
    assert_eq!(
        batches
            .iter()
            .map(|batch| batch.entries.len())
            .sum::<usize>(),
        5000
    );
    assert!(terminal.cleanup_complete);
}

#[tokio::test(flavor = "current_thread")]
async fn windows_long_source_path_uses_original_product_open_test_and_cleanup() {
    let (mut fixture, client, bridge) = Fixture::new().await;
    fixture.retain_inputs = true;
    std::fs::write(fixture.root.join("package/capture-terminal"), b"")
        .expect("capture original production cleanup terminal");
    let mut directory = fixture.root.join("long-source");
    let component = format!("长路径目录{}", "a".repeat(42));
    while directory
        .as_os_str()
        .to_str()
        .expect("Unicode long directory")
        .encode_utf16()
        .count()
        < 420
    {
        directory.push(&component);
    }
    std::fs::create_dir_all(&directory).expect("owned CJK nested long path");
    let source = directory.join("源归档.zip");
    std::fs::copy(fixture.root.join("literal.zip"), &source)
        .expect("independent tiny ZIP at long source");
    let path = source.to_str().expect("Unicode source path").to_owned();
    let utf16_length = path.encode_utf16().count();
    assert!(utf16_length > 420);
    assert!(!path.starts_with(r"\\?\"));
    let original_bytes = std::fs::read(&source).expect("independent ZIP bytes");
    let connection = bridge
        .connection_generation()
        .expect("original typed pipe connection");
    let mut terminals = Vec::new();
    for action in [ArchiveReadAction::Open, ArchiveReadAction::Test] {
        let mut command = fixture.request(action);
        command.path = Some(path.clone());
        let operation_id = command.operation_id;
        let operation = client
            .start(command)
            .expect("ordinary local long-path product admission");
        let (batches, terminal) = consume(&client, &operation).await;
        assert_eq!(
            terminal.status,
            ArchiveReadStatus::Succeeded,
            "{:?}",
            terminal.code
        );
        assert!(terminal.cleanup_complete);
        assert_eq!(bridge.connection_generation(), Some(connection));
        if action == ArchiveReadAction::Open {
            assert_eq!((terminal.entry_count, terminal.batch_count), (3, 2));
            let entries = batches
                .iter()
                .flat_map(|batch| &batch.entries)
                .collect::<Vec<_>>();
            assert_eq!(
                entries
                    .iter()
                    .map(|entry| entry.name.as_str())
                    .collect::<Vec<_>>(),
                ["alpha.txt", "目录/", "目录/你好.txt"]
            );
            assert_eq!(
                entries
                    .iter()
                    .map(|entry| entry.ordinal)
                    .collect::<Vec<_>>(),
                [0, 1, 2]
            );
            assert_eq!(
                entries.iter().map(|entry| entry.size).collect::<Vec<_>>(),
                [Some(6), Some(0), Some(6)]
            );
            assert_eq!(entries[2].provenance, "Utf8Flag");
            assert_eq!(entries[2].codepage, Some(65001));
            assert!(entries.iter().all(|entry| entry.warning_codes.is_empty()));
            assert_eq!(batches[0].snapshot_id, batches[1].snapshot_id);
            assert!(!batches[0].snapshot_id.is_nil());
            assert!(batches.iter().all(|batch| batch.format == "Zip"));
        } else {
            assert!(batches.is_empty());
            assert_eq!((terminal.files_verified, terminal.bytes_verified), (2, 12));
        }
        let remote: ArchiveReadTerminal = serde_json::from_slice(
            &std::fs::read(
                fixture
                    .root
                    .join(format!("package/terminal-{operation_id}.json")),
            )
            .expect("original remote terminal persisted before event"),
        )
        .expect("typed remote terminal");
        assert_eq!(terminal, remote);
        assert_eq!(
            std::fs::read(&source).expect("preserved source"),
            original_bytes
        );
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(&source)
                .is_ok(),
            "original source lease released only after cleanup"
        );
        terminals.push(terminal);
    }
    for internal in [
        r"\\?\C:\private.zip",
        r"\\?\Volume{00000000-0000-0000-0000-000000000000}\private.zip",
        r"\\server\share\private.zip",
        r"\??\C:\private.zip",
    ] {
        let mut command = fixture.request(ArchiveReadAction::Open);
        command.path = Some(internal.to_owned());
        assert!(matches!(
            client.start(command),
            Err(ArchiveReadError::InvalidRequest)
        ));
    }
    std::fs::remove_file(fixture.root.join("package/NxTools.ArchiveWorker.exe"))
        .expect("exact owned helper image released after actual Job cleanup");
    fixture
        .endpoint
        .as_ref()
        .expect("original endpoint")
        .set_connected_for_contract_test(false);
    fixture.server.take().expect("original server").abort();
    fixture.endpoint.take();
    tokio::time::timeout(Duration::from_secs(5), async {
        while fixture
            .child
            .as_mut()
            .expect("same test host")
            .try_wait()
            .expect("host wait")
            .is_none()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("original pipe EOF drains host without forced termination");
    assert!(
        fixture
            .child
            .as_mut()
            .expect("same test host")
            .try_wait()
            .expect("host exit")
            .expect("exited")
            .success()
    );
    let report = serde_json::json!({"schema":1,"status":"passed","source_utf16":utf16_length,"source_utf8":path.len(),
        "component_utf16":component.encode_utf16().count(),"same_connection":true,"original_remote_terminals_equal":true,
        "source_retained":true,"helper_released":true,"host_exited":true,"terminals":terminals});
    let evidence = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fixture.root.join("long-path-evidence.json"))
        .expect("new retained evidence");
    serde_json::to_writer_pretty(evidence, &report).expect("save path-free long-source facts");
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_credit_cancel_stops_batches_then_original_connection_readmits_after_cleanup() {
    let (mut fixture, client, bridge) = Fixture::new().await;
    fixture.retain_inputs = true;
    std::fs::write(fixture.root.join("package/capture-terminal"), b"")
        .expect("original terminal capture");
    let generation = bridge.connection_generation();
    let mut request = fixture.request(ArchiveReadAction::Open);
    request.limits = Some(ArchiveReadLimits {
        ack_timeout_ms: 10000,
        ..Default::default()
    });
    let id = request.operation_id;
    let operation = client.start(request).expect("original product admission");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            client.poll().await.expect("original typed event");
            if let Some(batch) = operation.take_batch() {
                assert_eq!(batch.sequence, 0);
                break;
            }
        }
    })
    .await
    .expect("actual first metadata credit boundary");
    assert!(operation.terminal().is_none());
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_err()
    );
    let issued = std::time::Instant::now();
    operation
        .cancel()
        .expect("caller cancel admission is not worker reception");
    let (later, terminal) = consume(&client, &operation).await;
    let caller_terminal_us = issued.elapsed().as_micros();
    assert!(
        later.is_empty(),
        "cancelled credit must not issue another business batch"
    );
    assert_eq!(terminal.code, Some(ArchiveReadError::Cancelled));
    assert!(terminal.cleanup_complete);
    let remote: ArchiveReadTerminal = serde_json::from_slice(
        &std::fs::read(fixture.root.join(format!("package/terminal-{id}.json")))
            .expect("original remote cleanup"),
    )
    .expect("terminal");
    assert_eq!(terminal, remote);
    assert_eq!(bridge.connection_generation(), generation);
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.root.join("literal.zip"))
            .is_ok()
    );
    let fresh = client
        .start(fixture.request(ArchiveReadAction::Test))
        .expect("fresh same-connection readmission after cleanup");
    let (_, success) = consume(&client, &fresh).await;
    assert_eq!(success.status, ArchiveReadStatus::Succeeded);
    assert!(success.cleanup_complete);
    assert_eq!((success.files_verified, success.bytes_verified), (2, 12));
    assert_eq!(bridge.connection_generation(), generation);
    std::fs::remove_file(fixture.root.join("package/NxTools.ArchiveWorker.exe"))
        .expect("actual helper lease released");
    fixture
        .endpoint
        .as_ref()
        .expect("endpoint")
        .set_connected_for_contract_test(false);
    fixture.server.take().expect("server").abort();
    tokio::time::timeout(Duration::from_secs(5), async {
        while fixture
            .child
            .as_mut()
            .expect("original host")
            .try_wait()
            .expect("host wait")
            .is_none()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("original host EOF/drain");
    assert!(
        fixture
            .child
            .as_mut()
            .expect("host")
            .try_wait()
            .expect("wait")
            .expect("exit")
            .success()
    );
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fixture.root.join("cancel-boundary-evidence.json"))
        .expect("additive evidence");
    serde_json::to_writer_pretty(file, &serde_json::json!({"operation_id":id,"caller_terminal_us":caller_terminal_us,
        "worker_receive_observe_times_measured_here":false,"same_connection":true,"later_batches":later.len(),"terminal":terminal,"followup":success,
        "source_released":true,"helper_released":true,"host_exited":true})).expect("save original product evidence");
}
