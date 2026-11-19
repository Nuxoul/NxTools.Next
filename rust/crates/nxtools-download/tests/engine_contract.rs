mod support;

use std::io::{Seek, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, Weekday};
use nxtools_core::{
    DownloadCategorySettings, DownloadDuplicateStrategySettings, DownloadSettings,
    NetworkProxyMode, NetworkProxySettings, NetworkSettings,
};
use nxtools_download::{
    BrowserBlob, BrowserDownloadSubmission, DOWNLOAD_COMMAND_PROTOCOL_VERSION, DashDownloadRequest,
    DownloadAdaptiveSegmentPolicy, DownloadBatchAction, DownloadCertificatePolicy, DownloadCommand,
    DownloadCommandEnvelope, DownloadCommandOutcome, DownloadDailySchedule,
    DownloadDuplicateStrategy, DownloadEngine, DownloadEvent, DownloadHttpPolicy, DownloadQuery,
    DownloadQueueAction, DownloadQueuePlacement, DownloadRequest, DownloadRequestHeader,
    DownloadRequestMedia, DownloadSchedule, DownloadScheduleMoment, DownloadSegmentState,
    DownloadStatus, DownloadSubmitMode, DownloadTask, DownloadTaskAction, DownloadWeekday,
    HlsDownloadRequest, NetworkPolicy, RequestOrigin, SqliteDownloadStore,
    daily_schedule_window_contains,
};
use rcgen::{CertifiedKey, generate_simple_self_signed};
use support::{DeterministicHttpServer, concurrent_fixture_bytes, imbalanced_fixture_bytes};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

fn request(server: &DeterministicHttpServer, save_directory: PathBuf) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://{}/range", server.address()),
        save_directory,
        save_directory_explicit: false,
        file_name: Some("engine-fixture.bin".to_owned()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    }
}

#[test]
fn category_directory_and_duplicate_name_are_reserved_atomically_at_acceptance() {
    let output = tempfile::tempdir().expect("create organization output directory");
    let documents = output.path().join("Documents");
    std::fs::create_dir_all(&documents).expect("create category directory");
    std::fs::write(documents.join("report.pdf"), b"existing").expect("seed occupied target");
    let settings = DownloadSettings {
        default_directory: output.path().to_string_lossy().into_owned(),
        automatic_categories_enabled: true,
        categories: vec![DownloadCategorySettings {
            id: "documents".into(),
            name: "文档".into(),
            enabled: true,
            save_directory: "Documents".into(),
            extensions: vec!["pdf".into()],
            url_patterns: Vec::new(),
        }],
        duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
        ..DownloadSettings::default()
    };
    let engine = DownloadEngine::open_in_memory().expect("open organization engine");
    engine
        .update_download_organization_policy(&settings)
        .expect("apply organization settings");

    let make_request = || DownloadRequest {
        source_url: "https://example.test/report.pdf".into(),
        save_directory: output.path().to_owned(),
        save_directory_explicit: false,
        file_name: Some("report.pdf".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    };

    let barrier = Arc::new(std::sync::Barrier::new(3));
    let first_engine = engine.clone();
    let first_barrier = Arc::clone(&barrier);
    let first_request = make_request();
    let first_thread = std::thread::spawn(move || {
        first_barrier.wait();
        first_engine.enqueue(first_request, RequestOrigin::Ui)
    });
    let second_engine = engine.clone();
    let second_barrier = Arc::clone(&barrier);
    let second_request = make_request();
    let second_thread = std::thread::spawn(move || {
        second_barrier.wait();
        second_engine.enqueue(second_request, RequestOrigin::Browser)
    });
    barrier.wait();
    let first_id = first_thread
        .join()
        .expect("join first concurrent acceptance")
        .expect("accept first categorized task");
    let second_id = second_thread
        .join()
        .expect("join second concurrent acceptance")
        .expect("accept second categorized task");
    let first = engine
        .task(first_id)
        .expect("load first categorized task")
        .expect("first categorized task exists");
    assert_eq!(first.task.request().save_directory, documents);
    assert_eq!(
        first.task.request().category_id.as_deref(),
        Some("documents")
    );
    assert_eq!(first.task.request().category_name.as_deref(), Some("文档"));
    assert_eq!(
        first.task.request().duplicate_strategy,
        DownloadDuplicateStrategy::AddNumbered
    );

    let second = engine
        .task(second_id)
        .expect("load second categorized task")
        .expect("second categorized task exists");
    let mut names = [
        first
            .task
            .request()
            .file_name
            .as_deref()
            .unwrap_or_default(),
        second
            .task
            .request()
            .file_name
            .as_deref()
            .unwrap_or_default(),
    ];
    names.sort_unstable();
    assert_eq!(names, ["report (1).pdf", "report (2).pdf"]);
}

#[tokio::test]
async fn authoritative_probe_name_is_frozen_before_classification_and_reservation() {
    let output = tempfile::tempdir().expect("create derived-name directory");
    let server = DeterministicHttpServer::start();
    let engine = DownloadEngine::open_in_memory().expect("open derived-name engine");
    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: output.path().to_string_lossy().into_owned(),
            automatic_categories_enabled: true,
            categories: vec![DownloadCategorySettings {
                id: "programs".into(),
                name: "程序".into(),
                enabled: true,
                save_directory: "Programs".into(),
                extensions: vec!["bin".into()],
                url_patterns: Vec::new(),
            }],
            ..DownloadSettings::default()
        })
        .expect("apply derived-name policy");
    let mut first = request(&server, output.path().to_owned());
    first.source_url = format!("http://{}/named/from-url.bin", server.address());
    first.file_name = None;
    let first_id = engine
        .submit(first.clone(), RequestOrigin::Cli)
        .expect("accept first unnamed task");
    engine
        .wait_for_terminal(first_id, Duration::from_secs(2))
        .await
        .expect("complete first server-named task");
    let second_id = engine
        .submit(first, RequestOrigin::Browser)
        .expect("accept second unnamed task");
    engine
        .wait_for_terminal(second_id, Duration::from_secs(2))
        .await
        .expect("complete second server-named task");
    let first = engine
        .task(first_id)
        .expect("read first server-named task")
        .expect("first server-named task exists");
    let second = engine
        .task(second_id)
        .expect("read second server-named task")
        .expect("second server-named task exists");
    assert_eq!(
        first.task.request().category_id.as_deref(),
        Some("programs")
    );
    assert_eq!(
        first.task.request().file_name.as_deref(),
        Some("server-name.bin")
    );
    assert_eq!(
        second.task.request().file_name.as_deref(),
        Some("server-name (1).bin")
    );
}

#[test]
fn reject_duplicate_strategy_does_not_create_a_task() {
    let output = tempfile::tempdir().expect("create reject output directory");
    std::fs::write(output.path().join("occupied.bin"), b"existing").expect("seed rejected target");
    let settings = DownloadSettings {
        default_directory: output.path().to_string_lossy().into_owned(),
        duplicate_strategy: DownloadDuplicateStrategySettings::Reject,
        ..DownloadSettings::default()
    };
    let engine = DownloadEngine::open_in_memory().expect("open reject engine");
    engine
        .update_download_organization_policy(&settings)
        .expect("apply reject policy");
    let mut occupied = request(&DeterministicHttpServer::start(), output.path().to_owned());
    occupied.file_name = Some("occupied.bin".into());

    let error = engine
        .enqueue(occupied, RequestOrigin::Cli)
        .expect_err("occupied target must be rejected");
    assert!(matches!(
        error,
        nxtools_download::DownloadEngineError::DownloadOrganizationPolicy(
            nxtools_download::DownloadOrganizationPolicyError::TargetOccupied
        )
    ));
    assert!(
        engine
            .tasks()
            .expect("read rejected engine tasks")
            .is_empty()
    );
}

#[test]
fn manual_directory_wins_and_an_unavailable_explicit_category_is_rejected() {
    let root = tempfile::tempdir().expect("create explicit category root");
    let manual = tempfile::tempdir().expect("create manual target root");
    let engine = DownloadEngine::open_in_memory().expect("open explicit category engine");
    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: root.path().to_string_lossy().into_owned(),
            categories: vec![DownloadCategorySettings {
                id: "documents".into(),
                name: "文档".into(),
                enabled: true,
                save_directory: "Documents".into(),
                extensions: vec!["pdf".into()],
                url_patterns: Vec::new(),
            }],
            ..DownloadSettings::default()
        })
        .expect("apply explicit category policy");
    let mut selected = request(&DeterministicHttpServer::start(), manual.path().to_owned());
    selected.file_name = Some("manual.pdf".into());
    selected.save_directory_explicit = true;
    selected.category_id = Some("documents".into());
    let task_id = engine
        .enqueue(selected, RequestOrigin::Ui)
        .expect("accept explicit category with manual directory");
    let task = engine
        .task(task_id)
        .expect("read explicitly categorized task")
        .expect("explicitly categorized task exists");
    assert_eq!(task.task.request().save_directory, manual.path());
    assert_eq!(task.task.request().category_name.as_deref(), Some("文档"));

    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: root.path().to_string_lossy().into_owned(),
            categories: Vec::new(),
            ..DownloadSettings::default()
        })
        .expect("remove category from later settings");
    let mut stale = request(&DeterministicHttpServer::start(), root.path().to_owned());
    stale.category_id = Some("documents".into());
    let error = engine
        .enqueue(stale, RequestOrigin::Browser)
        .expect_err("stale explicit selection must not silently downgrade");
    assert!(matches!(
        error,
        nxtools_download::DownloadEngineError::DownloadOrganizationPolicy(
            nxtools_download::DownloadOrganizationPolicyError::CategoryUnavailable
        )
    ));
}

#[test]
fn browser_batch_reserves_same_name_items_in_submission_order() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("create batch organization directory");
    let engine = DownloadEngine::open_in_memory().expect("open batch organization engine");
    let same_name = request(&server, output.path().to_owned());
    let submissions = vec![
        BrowserDownloadSubmission {
            handoff_id: uuid::Uuid::new_v4(),
            request: same_name.clone(),
        },
        BrowserDownloadSubmission {
            handoff_id: uuid::Uuid::new_v4(),
            request: same_name,
        },
    ];
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit { submissions },
    });
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = result.outcome else {
        panic!("same-name browser batch was not accepted");
    };
    let names = results
        .into_iter()
        .map(|result| {
            let task_id = result.task_id.expect("accepted batch item has task ID");
            engine
                .task(task_id)
                .expect("read batch task")
                .expect("batch task exists")
                .task
                .request()
                .file_name
                .clone()
                .expect("batch task has file name")
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["engine-fixture.bin", "engine-fixture (1).bin"]);
}

#[test]
fn resolved_category_destination_survives_restart_and_later_policy_changes() {
    let output = tempfile::tempdir().expect("create restart organization directory");
    let database = output.path().join("downloads.sqlite3");
    let category_a = DownloadCategorySettings {
        id: "documents-a".into(),
        name: "文档 A".into(),
        enabled: true,
        save_directory: "A".into(),
        extensions: vec!["pdf".into()],
        url_patterns: Vec::new(),
    };
    let engine = DownloadEngine::open(&database).expect("open organization database");
    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: output.path().to_string_lossy().into_owned(),
            automatic_categories_enabled: true,
            categories: vec![category_a],
            ..DownloadSettings::default()
        })
        .expect("apply first category policy");
    let server = DeterministicHttpServer::start();
    let mut categorized = request(&server, output.path().to_owned());
    categorized.file_name = Some("restart.pdf".into());
    let task_id = engine
        .enqueue(categorized, RequestOrigin::Cli)
        .expect("accept categorized restart task");
    drop(engine);

    let reopened = DownloadEngine::open(&database).expect("reopen organization database");
    reopened
        .update_download_organization_policy(&DownloadSettings {
            default_directory: output.path().to_string_lossy().into_owned(),
            automatic_categories_enabled: true,
            categories: vec![DownloadCategorySettings {
                id: "documents-b".into(),
                name: "文档 B".into(),
                enabled: true,
                save_directory: "B".into(),
                extensions: vec!["pdf".into()],
                url_patterns: Vec::new(),
            }],
            ..DownloadSettings::default()
        })
        .expect("apply later category policy");
    let restored = reopened
        .task(task_id)
        .expect("read restored categorized task")
        .expect("restored categorized task exists");
    assert_eq!(
        restored.task.request().save_directory,
        output.path().join("A")
    );
    assert_eq!(
        restored.task.request().category_id.as_deref(),
        Some("documents-a")
    );
}

#[test]
fn automatic_category_requires_all_configured_rules_and_prefers_url_specific_rule() {
    let output = tempfile::tempdir().expect("create category precedence directory");
    let engine = DownloadEngine::open_in_memory().expect("open category precedence engine");
    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: output.path().to_string_lossy().into_owned(),
            automatic_categories_enabled: true,
            categories: vec![
                DownloadCategorySettings {
                    id: "broad-pdf".into(),
                    name: "全部 PDF".into(),
                    enabled: true,
                    save_directory: "Documents".into(),
                    extensions: vec!["pdf".into()],
                    url_patterns: Vec::new(),
                },
                DownloadCategorySettings {
                    id: "wildcard-pdf".into(),
                    name: "任意来源 PDF".into(),
                    enabled: true,
                    save_directory: "Wildcard".into(),
                    extensions: vec!["pdf".into()],
                    url_patterns: vec!["*".into()],
                },
                DownloadCategorySettings {
                    id: "secure-pdf".into(),
                    name: "安全 PDF".into(),
                    enabled: true,
                    save_directory: "Secure".into(),
                    extensions: vec!["pdf".into()],
                    url_patterns: vec!["https://secure.example.test/*".into()],
                },
            ],
            ..DownloadSettings::default()
        })
        .expect("apply category precedence policy");
    let mut secure = request(&DeterministicHttpServer::start(), output.path().to_owned());
    secure.source_url = "https://secure.example.test/report.pdf".into();
    secure.file_name = Some("report.pdf".into());
    let secure_id = engine
        .enqueue(secure, RequestOrigin::Browser)
        .expect("accept URL-specific category task");
    let secure = engine
        .task(secure_id)
        .expect("read URL-specific task")
        .expect("URL-specific task exists");
    assert_eq!(
        secure.task.request().category_id.as_deref(),
        Some("secure-pdf")
    );

    let mut ordinary = request(&DeterministicHttpServer::start(), output.path().to_owned());
    ordinary.source_url = "https://public.example.test/report.pdf".into();
    ordinary.file_name = Some("public.pdf".into());
    let ordinary_id = engine
        .enqueue(ordinary, RequestOrigin::Ui)
        .expect("accept broad category task");
    let ordinary = engine
        .task(ordinary_id)
        .expect("read broad category task")
        .expect("broad category task exists");
    assert_eq!(
        ordinary.task.request().category_id.as_deref(),
        Some("wildcard-pdf")
    );
}

#[test]
fn category_relative_directory_cannot_escape_the_selected_download_root() {
    let output = tempfile::tempdir().expect("create category traversal directory");
    let engine = DownloadEngine::open_in_memory().expect("open category traversal engine");
    let error = engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: output.path().to_string_lossy().into_owned(),
            automatic_categories_enabled: true,
            categories: vec![DownloadCategorySettings {
                id: "escape".into(),
                name: "越界".into(),
                enabled: true,
                save_directory: r"..\outside".into(),
                extensions: vec!["bin".into()],
                url_patterns: Vec::new(),
            }],
            ..DownloadSettings::default()
        })
        .expect_err("parent traversal category directory must be rejected");
    assert!(matches!(
        error,
        nxtools_download::DownloadEngineError::DownloadOrganizationPolicy(
            nxtools_download::DownloadOrganizationPolicyError::InvalidCategoryDirectory
        )
    ));
}

#[cfg(windows)]
#[test]
fn category_directory_cannot_escape_through_a_real_junction() {
    let output = tempfile::tempdir().expect("create category junction root");
    let outside = tempfile::tempdir().expect("create category junction target");
    let link = output.path().join("Linked");
    let status = Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            link.to_string_lossy().as_ref(),
            outside.path().to_string_lossy().as_ref(),
        ])
        .status()
        .expect("create category junction");
    assert!(status.success());
    let engine = DownloadEngine::open_in_memory().expect("open category junction engine");
    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: output.path().to_string_lossy().into_owned(),
            automatic_categories_enabled: true,
            categories: vec![DownloadCategorySettings {
                id: "junction".into(),
                name: "链接".into(),
                enabled: true,
                save_directory: "Linked\\Nested".into(),
                extensions: vec!["bin".into()],
                url_patterns: Vec::new(),
            }],
            ..DownloadSettings::default()
        })
        .expect("configuration is lexically valid");
    let mut candidate = request(&DeterministicHttpServer::start(), output.path().to_owned());
    candidate.file_name = Some("artifact.bin".into());
    let error = engine
        .enqueue(candidate, RequestOrigin::Ui)
        .expect_err("junction escape must be rejected at the real filesystem boundary");
    let rejected_escape = matches!(
        &error,
        nxtools_download::DownloadEngineError::DownloadOrganizationPolicy(
            nxtools_download::DownloadOrganizationPolicyError::InvalidCategoryDirectory
        )
    ) || matches!(
        &error,
        nxtools_download::DownloadEngineError::DownloadOrganizationPolicy(
            nxtools_download::DownloadOrganizationPolicyError::Io(native)
        ) if native.raw_os_error() == Some(448)
    );
    assert!(
        rejected_escape,
        "isolated junction rejection returned {error:?}"
    );
    assert!(
        engine.tasks().expect("read isolated tasks").is_empty(),
        "rejected junction must not admit a task"
    );
    assert!(!outside.path().join("Nested").exists());
}

async fn start_self_signed_https_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])
            .expect("generate self-signed certificate");
    let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(signing_key.serialize_der()));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()], private_key)
        .expect("build TLS fixture config");
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind TLS fixture");
    let address = listener.local_addr().expect("read TLS fixture address");
    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut stream) = acceptor.accept(stream).await else {
                    return;
                };
                let mut request = Vec::new();
                let mut chunk = [0_u8; 1024];
                while request.len() < 16 * 1024 {
                    let Ok(read) = stream.read(&mut chunk).await else {
                        return;
                    };
                    if read == 0 {
                        return;
                    }
                    request.extend_from_slice(&chunk[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request);
                let method = request.split_whitespace().next().unwrap_or("");
                let range = request.lines().find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("range")
                            .then(|| value.trim().to_owned())
                    })
                });
                let (status, extra_headers, body): (&str, String, &[u8]) = match range.as_deref() {
                    Some("bytes=0-0") => (
                        "206 Partial Content",
                        "Content-Range: bytes 0-0/16\r\nAccept-Ranges: bytes\r\n".into(),
                        b"0",
                    ),
                    Some("bytes=0-15") => (
                        "206 Partial Content",
                        "Content-Range: bytes 0-15/16\r\nAccept-Ranges: bytes\r\n".into(),
                        b"0123456789abcdef",
                    ),
                    _ => (
                        "200 OK",
                        "Accept-Ranges: bytes\r\n".into(),
                        b"0123456789abcdef",
                    ),
                };
                let response_body = if method == "HEAD" { &[][..] } else { body };
                let response = format!(
                    "HTTP/1.1 {status}\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.write_all(response_body).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    (address, task)
}

#[tokio::test]
async fn download_defaults_merge_at_acceptance_and_explicit_request_context_wins() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("create output directory");
    let database = output.path().join("downloads.sqlite3");
    let settings = DownloadSettings {
        default_request_headers: concat!(
            "-H 'X-NxTools-Default: default'\n",
            "X-NxTools-Precedence: default\n",
            "Referer: https://default.example/"
        )
        .into(),
        default_cookie: "default-cookie=1".into(),
        ..DownloadSettings::default()
    };
    let policy =
        DownloadHttpPolicy::from_download_settings(&settings).expect("parse download HTTP policy");
    let engine = DownloadEngine::open_with_policies(
        &database,
        NetworkPolicy::new(NetworkSettings::default()).expect("create network policy"),
        policy,
    )
    .expect("open engine");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: format!(
                    "http://{}/stage6/request-context-redirect",
                    server.address()
                ),
                save_directory: output.path().to_owned(),
                save_directory_explicit: false,
                file_name: Some("request-context.bin".into()),
                headers: vec![
                    DownloadRequestHeader {
                        name: "X-NxTools-Explicit".into(),
                        value: "explicit".into(),
                    },
                    DownloadRequestHeader {
                        name: "x-nxtools-precedence".into(),
                        value: "task".into(),
                    },
                ],
                cookie: Some("task-cookie=1".into()),
                referrer: Some("https://task.example/".into()),
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Ui,
        )
        .expect("submit request-context task");
    let changed_settings = DownloadSettings {
        default_request_headers: "X-NxTools-Default: changed-after-acceptance".into(),
        default_cookie: "changed-cookie=1".into(),
        ..DownloadSettings::default()
    };
    drop(engine);
    let reopened = DownloadEngine::open_with_policies(
        &database,
        NetworkPolicy::new(NetworkSettings::default()).expect("recreate network policy"),
        DownloadHttpPolicy::from_download_settings(&changed_settings)
            .expect("parse later request defaults"),
    )
    .expect("reopen engine with later defaults");
    reopened
        .resume(task_id)
        .expect("resume accepted request-context task after reopen");

    let terminal = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for request-context task");
    assert_eq!(terminal.task.status(), DownloadStatus::Completed);
    assert_eq!(
        std::fs::read(output.path().join("request-context.bin")).expect("read artifact"),
        b"0123456789abcdef"
    );
    let persisted = terminal.task.request();
    assert_eq!(persisted.cookie.as_deref(), Some("task-cookie=1"));
    assert!(persisted.headers.iter().any(|header| {
        header.name.eq_ignore_ascii_case("x-nxtools-default") && header.value == "default"
    }));
    assert!(
        persisted
            .headers
            .iter()
            .all(|header| !header.name.eq_ignore_ascii_case("referer"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn certificate_policy_is_strict_by_default_and_explicit_opt_in_is_frozen_into_task() {
    let (address, server_task) = start_self_signed_https_server().await;
    let output = tempfile::tempdir().expect("create TLS output directory");
    let database = output.path().join("tls-downloads.sqlite3");
    let source_url = format!("https://{address}/fixture.bin");
    let make_request = |file_name: &str| DownloadRequest {
        source_url: source_url.clone(),
        save_directory: output.path().to_owned(),
        save_directory_explicit: false,
        file_name: Some(file_name.into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    };

    let strict_engine = DownloadEngine::open_in_memory().expect("open strict engine");
    let strict = strict_engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Probe {
            request: make_request("strict.bin"),
        },
    });
    assert!(
        matches!(strict.outcome, DownloadCommandOutcome::Rejected { .. }),
        "strict default unexpectedly trusted a self-signed certificate"
    );

    let settings = DownloadSettings {
        accept_invalid_certificates: true,
        ..DownloadSettings::default()
    };
    let permissive_engine = DownloadEngine::open_with_policies(
        &database,
        NetworkPolicy::new(NetworkSettings::default()).expect("create network policy"),
        DownloadHttpPolicy::from_download_settings(&settings).expect("create HTTP policy"),
    )
    .expect("open permissive engine");
    let probe = permissive_engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Probe {
            request: make_request("permissive.bin"),
        },
    });
    assert!(matches!(probe.outcome, DownloadCommandOutcome::Probe(_)));

    let task_id = permissive_engine
        .enqueue(make_request("permissive.bin"), RequestOrigin::Ui)
        .expect("submit permissive TLS task");
    drop(permissive_engine);
    let reopened = DownloadEngine::open(&database).expect("reopen task under strict live policy");
    reopened
        .resume(task_id)
        .expect("resume accepted permissive TLS task");
    let terminal = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for permissive TLS task");
    assert_eq!(terminal.task.status(), DownloadStatus::Completed);
    assert_eq!(
        terminal.task.request().certificate_policy,
        DownloadCertificatePolicy::AcceptInvalid
    );
    assert_eq!(
        std::fs::read(output.path().join("permissive.bin")).expect("read TLS artifact"),
        b"0123456789abcdef"
    );
    server_task.abort();
    let _ = server_task.await;
}

fn write_ffmpeg_fixture(path: &std::path::Path) {
    std::fs::write(
        path,
        r#"@echo off
if "%~1"=="-version" echo ffmpeg version NxTools-test& exit /b 0
set "first="
set "second="
set "previous="
:loop
if "%~1"=="" goto done
if not "%previous%"=="-i" goto not_input
if defined first goto set_second
set "first=%~1"
goto not_input
:set_second
if not defined second set "second=%~1"
:not_input
if "%previous%"=="-f" set "format=%~1"
set "previous=%~1"
set "output=%~1"
shift
goto loop
:done
if not "%format%"=="mp4" exit /b 9
echo out_time_us=500000
echo progress=continue
ping -n 2 127.0.0.1 >nul
echo out_time_us=1000000
echo progress=end
copy /b "%first%"+"%second%" "%output%" >nul
"#,
    )
    .expect("write fake FFmpeg boundary");
}

fn write_blocking_ffmpeg_fixture(path: &std::path::Path, block_version: bool) {
    let version_action = if block_version {
        r#"echo started>"%~dp0ffmpeg.started"
:wait_version
ping -n 2 127.0.0.1 >nul
goto wait_version"#
    } else {
        r#"echo ffmpeg version NxTools-test
exit /b 0"#
    };
    std::fs::write(
        path,
        format!(
            r#"@echo off
if "%~1"=="-version" goto version
echo started>"%~dp0ffmpeg.started"
:wait_mux
ping -n 2 127.0.0.1 >nul
goto wait_mux
:version
{version_action}
"#
        ),
    )
    .expect("write blocking FFmpeg boundary");
}

#[tokio::test]
async fn selected_dash_video_track_downloads_init_and_segments_in_manifest_order() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create DASH engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("video.mp4".into());
    dash_request.connection_limit = Some(2);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit selected DASH video");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for DASH terminal state");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "DASH video failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("video.mp4")).expect("read merged DASH artifact"),
        b"VINIT|V0|V1"
    );
    let observed = server.max_concurrent_range_transfers();
    assert!(
        observed >= 2,
        "expected DASH fragment concurrency, saw {observed}"
    );
    assert!(
        observed <= 2,
        "DASH exceeded configured connection limit: {observed}"
    );
}

#[tokio::test]
async fn paused_dash_video_reopens_from_durable_fragment_checkpoint() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create resumable DASH directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("resumed-video.mp4".into());
    dash_request.connection_limit = Some(1);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));
    server.hold_dash_first_media(true);

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit resumable DASH video");
    let initialization = destination
        .join(format!(".nxtools-dash-{}", task_id.as_uuid()))
        .join("00000000.fragment");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !initialization.exists() || server.dash_first_media_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "DASH initialization did not become durable before the media request"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause DASH video");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused DASH video");
    server.hold_dash_first_media(false);
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(initialization.exists());
    drop(engine);

    let reopened = DownloadEngine::open(&database).expect("reopen persistent download engine");
    reopened
        .resume(task_id)
        .expect("resume restored DASH video");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for resumed DASH video");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "resumed DASH failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("resumed-video.mp4")).expect("read resumed DASH artifact"),
        b"VINIT|V0|V1"
    );
    assert_eq!(
        server.dash_initialization_attempts(),
        1,
        "durable DASH initialization must not be requested again after reopen"
    );
}

#[tokio::test]
async fn completed_dash_fragment_with_wrong_length_is_redownloaded_after_reopen() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create DASH recovery directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open DASH engine");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("recovered.mp4".into());
    dash_request.connection_limit = Some(1);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));
    server.hold_dash_first_media(true);
    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit DASH");
    let initialization = destination
        .join(format!(".nxtools-dash-{}", task_id.as_uuid()))
        .join("00000000.fragment");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !initialization.exists() || server.dash_first_media_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "DASH initialization did not finish"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause DASH");
    assert_eq!(
        engine
            .wait_for_terminal(task_id, Duration::from_secs(2))
            .await
            .expect("wait for pause")
            .task
            .status(),
        DownloadStatus::Paused
    );
    server.hold_dash_first_media(false);
    drop(engine);
    std::fs::write(&initialization, b"CORRUPTED-LONGER-THAN-INIT")
        .expect("corrupt completed fragment without changing checkpoint");
    let reopened = DownloadEngine::open(&database).expect("reopen DASH engine");
    reopened.resume(task_id).expect("resume DASH");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for recovery");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("recovered.mp4")).expect("read output"),
        b"VINIT|V0|V1"
    );
    assert_eq!(
        server.dash_initialization_attempts(),
        2,
        "a corrupted completed checkpoint must be fetched again"
    );
}
#[tokio::test]
async fn reopened_dash_video_rejects_changed_track_identity_before_mixing_fragments() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create mutable DASH directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("changed-video.mp4".into());
    dash_request.connection_limit = Some(1);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));
    server.hold_dash_first_media(true);

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit mutable DASH video");
    let working = destination.join(format!(".nxtools-dash-{}", task_id.as_uuid()));
    let durable_initialization = working.join("00000000.fragment");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !durable_initialization.exists() || server.dash_first_media_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "DASH initialization did not become durable"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause mutable DASH video");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for mutable DASH pause");
    server.hold_dash_first_media(false);
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    drop(engine);

    server.advance_resource_revision();
    let reopened = DownloadEngine::open(&database).expect("reopen mutable DASH engine");
    reopened.resume(task_id).expect("resume mutable DASH video");
    let failed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for incompatible DASH terminal state");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(durable_initialization.exists());
    assert!(!destination.join("changed-video.mp4").exists());
}

#[tokio::test]
async fn reopened_dash_video_rejects_raw_timescale_changes_hidden_by_rounded_duration() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create raw-timescale DASH directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/timescale.mpd", server.address());
    dash_request.file_name = Some("timescale-video.mp4".into());
    dash_request.connection_limit = Some(1);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));
    server.hold_dash_first_media(true);

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit raw-timescale DASH video");
    let working = destination.join(format!(".nxtools-dash-{}", task_id.as_uuid()));
    let initialization = working.join("00000000.fragment");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !initialization.exists() || server.dash_first_media_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "raw-timescale DASH initialization did not become durable"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine
        .pause(task_id)
        .expect("pause raw-timescale DASH video");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for raw-timescale DASH pause");
    server.hold_dash_first_media(false);
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    drop(engine);

    server.advance_resource_revision();
    let reopened = DownloadEngine::open(&database).expect("reopen raw-timescale DASH engine");
    reopened
        .resume(task_id)
        .expect("resume raw-timescale DASH video");
    let failed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for raw-timescale incompatibility");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(initialization.exists());
    assert!(!destination.join("timescale-video.mp4").exists());
}

#[tokio::test]
async fn cancelling_a_paused_dash_task_removes_fragments_and_durable_plan() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create cancellable DASH directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("cancelled-video.mp4".into());
    dash_request.connection_limit = Some(1);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));
    server.hold_dash_first_media(true);
    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit cancellable DASH video");
    let working = destination.join(format!(".nxtools-dash-{}", task_id.as_uuid()));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !working.join("00000000.fragment").exists() || server.dash_first_media_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "DASH task did not checkpoint"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause DASH task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for DASH pause");
    server.hold_dash_first_media(false);
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    engine.cancel(task_id).expect("cancel paused DASH task");
    assert_eq!(
        engine
            .task(task_id)
            .expect("load task")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Cancelled
    );
    assert!(!working.exists());
    let connection = rusqlite::Connection::open(&database).expect("open DASH database");
    let plan_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM download_dash_plans WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
            |row| row.get(0),
        )
        .expect("count DASH plans");
    assert_eq!(plan_count, 0);
}

#[tokio::test]
async fn selected_dash_video_and_audio_tracks_are_muxed_by_a_bounded_ffmpeg_process() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create DASH mux directory");
    let destination = directory.path().join("artifacts");
    let ffmpeg = directory.path().join("ffmpeg-fixture.cmd");
    write_ffmpeg_fixture(&ffmpeg);
    let engine = DownloadEngine::open_in_memory_with_ffmpeg(&ffmpeg)
        .expect("open download engine with FFmpeg fixture");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("muxed.mp4".into());
    dash_request.connection_limit = Some(2);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: Some("a128".into()),
    }));

    let events = engine
        .take_event_receiver()
        .expect("attach finishing event receiver");
    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit selected DASH video and audio");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for DASH mux terminal state");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "DASH mux failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("muxed.mp4")).expect("read muxed DASH artifact"),
        b"VINIT|V0|V1AINIT|A0|A1"
    );
    let observed = std::iter::from_fn(|| events.try_recv().ok())
        .map(|event| serde_json::to_value(event.event).expect("serialize event"))
        .collect::<Vec<_>>();
    let mux = observed
        .iter()
        .position(|event| event["data"]["progress"]["finishing"]["stage"] == "muxing");
    let completed_event = observed
        .iter()
        .position(|event| event["data"]["task"]["status"] == "completed");
    assert!(
        matches!((mux, completed_event), (Some(mux), Some(done)) if mux < done),
        "mux progress must precede Completed: {observed:?}"
    );
    assert!(
        observed
            .iter()
            .any(|event| event["data"]["progress"]["finishing"]["completed"] == 1000000),
        "FFmpeg clock must reach engine event"
    );
    assert!(
        observed
            .iter()
            .any(|event| event["data"]["progress"]["finishing"]["stage"] == "publishing")
    );
}

#[tokio::test]
async fn missing_ffmpeg_fails_a_real_dual_track_task_with_actionable_settings_guidance() {
    const CHILD_MARKER: &str = "NXTOOLS_MISSING_FFMPEG_CHILD";
    if std::env::var_os(CHILD_MARKER).is_none() {
        let status = Command::new(std::env::current_exe().expect("locate engine contract process"))
            .args([
                "--exact",
                "missing_ffmpeg_fails_a_real_dual_track_task_with_actionable_settings_guidance",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_MARKER, "1")
            .env_remove("FFMPEG_PATH")
            .env("PATH", "")
            .status()
            .expect("run missing FFmpeg child");
        assert!(status.success());
        return;
    }

    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create missing FFmpeg DASH directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open engine without FFmpeg");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("missing-ffmpeg.mp4".into());
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: Some("a128".into()),
    }));

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit dual-track DASH without FFmpeg");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for missing FFmpeg terminal state");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    let message = failed.error.expect("missing FFmpeg fault").message;
    assert!(message.contains("下载设置 → 媒体工具"));
    assert!(message.contains("公开下载链接"));
    assert!(!destination.join("missing-ffmpeg.mp4").exists());
}

#[tokio::test]
async fn pausing_during_ffmpeg_detection_preserves_dash_fragments_without_publishing_output() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create FFmpeg detection pause directory");
    let destination = directory.path().join("artifacts");
    let ffmpeg = directory.path().join("ffmpeg-block-version.cmd");
    let marker = directory.path().join("ffmpeg.started");
    write_blocking_ffmpeg_fixture(&ffmpeg, true);
    let engine = DownloadEngine::open_in_memory_with_ffmpeg(&ffmpeg)
        .expect("open engine with blocking FFmpeg version fixture");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("paused-mux.mp4".into());
    dash_request.connection_limit = Some(2);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: Some("a128".into()),
    }));

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit FFmpeg detection pause task");
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !marker.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "FFmpeg version process did not start"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause FFmpeg detection");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("FFmpeg detection should be pause-responsive");
    assert_eq!(
        paused.task.status(),
        DownloadStatus::Paused,
        "FFmpeg detection pause failed: {:?}",
        paused.error
    );
    assert!(
        destination
            .join(format!(".nxtools-dash-{}", task_id.as_uuid()))
            .exists()
    );
    assert!(!destination.join("paused-mux.mp4").exists());
}

#[tokio::test]
async fn dash_finalize_cleanup_obstruction_fails_instead_of_losing_its_terminal() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create DASH finalize obstruction directory");
    let destination = directory.path().join("artifacts");
    let ffmpeg = directory.path().join("ffmpeg-block-cleanup.cmd");
    let marker = directory.path().join("ffmpeg.started");
    write_blocking_ffmpeg_fixture(&ffmpeg, true);
    let engine = DownloadEngine::open_in_memory_with_ffmpeg(&ffmpeg)
        .expect("open engine with blocked FFmpeg version fixture");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("cleanup-obstructed.mp4".into());
    dash_request.connection_limit = Some(2);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: Some("a128".into()),
    }));

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit DASH cleanup obstruction task");
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !marker.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "FFmpeg version process did not start"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let temporary_path = engine
        .task(task_id)
        .expect("read DASH cleanup task")
        .expect("DASH cleanup task exists")
        .temporary_path
        .expect("DASH cleanup task has temporary path");
    std::fs::create_dir_all(&temporary_path)
        .expect("obstruct temporary-file cleanup with directory");
    engine
        .pause(task_id)
        .expect("pause obstructed DASH finalization");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("cleanup obstruction should become a terminal failure");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(temporary_path.is_dir());
    assert!(!destination.join("cleanup-obstructed.mp4").exists());
}

#[tokio::test]
async fn cancelling_during_ffmpeg_mux_removes_dash_staging_and_output() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create FFmpeg mux cancel directory");
    let destination = directory.path().join("artifacts");
    let ffmpeg = directory.path().join("ffmpeg-block-mux.cmd");
    let marker = directory.path().join("ffmpeg.started");
    write_blocking_ffmpeg_fixture(&ffmpeg, false);
    let engine = DownloadEngine::open_in_memory_with_ffmpeg(&ffmpeg)
        .expect("open engine with blocking FFmpeg mux fixture");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("cancelled-mux.mp4".into());
    dash_request.connection_limit = Some(2);
    dash_request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: dash_request.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: Some("a128".into()),
    }));

    let task_id = engine
        .submit(dash_request, RequestOrigin::Ui)
        .expect("submit FFmpeg mux cancel task");
    let working = destination.join(format!(".nxtools-dash-{}", task_id.as_uuid()));
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !marker.exists() {
        let task = engine
            .task(task_id)
            .expect("read mux-start task")
            .expect("mux-start task exists");
        assert!(
            !matches!(
                task.task.status(),
                DownloadStatus::Failed | DownloadStatus::Completed | DownloadStatus::Cancelled
            ),
            "FFmpeg mux marker missing after terminal state: status={:?}, error={:?}",
            task.task.status(),
            task.error
        );
        assert!(
            std::time::Instant::now() < deadline,
            "FFmpeg mux process did not start"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.cancel(task_id).expect("cancel FFmpeg mux");
    let cancelled = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("FFmpeg mux should be cancel-responsive");
    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert!(!working.exists());
    assert!(!destination.join("cancelled-mux.mp4").exists());
}

#[tokio::test]
async fn cli_dash_url_auto_selects_tracks_and_persists_the_media_request() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create CLI DASH directory");
    let destination = directory.path().join("artifacts");
    let ffmpeg = directory.path().join("ffmpeg-fixture.cmd");
    write_ffmpeg_fixture(&ffmpeg);
    let engine = DownloadEngine::open_in_memory_with_ffmpeg(&ffmpeg)
        .expect("open download engine with FFmpeg fixture");
    let mut dash_request = request(&server, destination.clone());
    dash_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    dash_request.file_name = Some("cli-auto.mp4".into());
    dash_request.media = None;

    let task_id = engine
        .submit(dash_request, RequestOrigin::Cli)
        .expect("submit plain DASH URL from CLI");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for CLI DASH terminal state");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "automatic DASH selection failed: {:?}",
        completed.error
    );
    assert!(matches!(
        completed.task.request().media,
        Some(DownloadRequestMedia::Dash(DashDownloadRequest {
            video_representation_id: Some(_),
            audio_representation_id: Some(_),
            ..
        }))
    ));
    assert_eq!(
        std::fs::read(destination.join("cli-auto.mp4")).expect("read automatic DASH artifact"),
        b"VINIT|V0|V1AINIT|A0|A1"
    );

    let mut browser_request = request(&server, destination.clone());
    browser_request.source_url = format!("http://{}/dash/static.mpd", server.address());
    browser_request.file_name = Some("manifest.mpd".into());
    browser_request.media = None;
    let browser_task = engine
        .submit(browser_request, RequestOrigin::Browser)
        .expect("submit plain DASH URL from browser handoff");
    let browser_completed = engine
        .wait_for_terminal(browser_task, Duration::from_secs(5))
        .await
        .expect("wait for browser DASH terminal state");
    assert_eq!(
        browser_completed.task.status(),
        DownloadStatus::Completed,
        "browser automatic DASH selection failed: {:?}",
        browser_completed.error
    );
    assert!(matches!(
        browser_completed.task.request().media,
        Some(DownloadRequestMedia::Dash(DashDownloadRequest {
            video_representation_id: Some(_),
            audio_representation_id: Some(_),
            ..
        }))
    ));
    assert_eq!(
        browser_completed.task.request().file_name.as_deref(),
        Some("manifest.mp4")
    );
    assert_eq!(
        std::fs::read(destination.join("manifest.mp4"))
            .expect("read browser automatic DASH artifact"),
        b"VINIT|V0|V1AINIT|A0|A1"
    );
}

#[tokio::test]
async fn browser_hls_url_auto_selects_media_and_replaces_the_manifest_extension() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create browser HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/high/index.m3u8", server.address());
    hls_request.file_name = Some("captured.m3u8".into());
    hls_request.media = None;

    let task_id = engine
        .submit(hls_request, RequestOrigin::Browser)
        .expect("submit plain HLS URL from browser handoff");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for automatic browser HLS terminal state");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "automatic browser HLS failed: {:?}",
        completed.error
    );
    assert!(matches!(
        completed.task.request().media,
        Some(DownloadRequestMedia::Hls(_))
    ));
    assert_eq!(
        completed.task.request().file_name.as_deref(),
        Some("captured.ts")
    );
    assert_eq!(
        std::fs::read(destination.join("captured.ts")).expect("read automatic HLS artifact"),
        b"segment-ten|segment-eleven|segment-twelve"
    );
}

#[tokio::test]
async fn selected_hls_vod_downloads_segments_and_commits_them_in_playlist_order() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create HLS engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("movie.ts".into());
    hls_request.connection_limit = Some(2);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/high/index.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit selected HLS VOD");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for HLS VOD terminal state");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "HLS VOD failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("movie.ts")).expect("read merged HLS artifact"),
        b"segment-ten|segment-eleven|segment-twelve"
    );
}

#[tokio::test]
async fn selected_aes128_hls_vod_decrypts_rotation_and_caches_shared_keys() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create AES-128 HLS engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("encrypted.ts".into());
    hls_request.connection_limit = Some(3);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/index.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit AES-128 HLS VOD");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for AES-128 HLS terminal state");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "AES-128 HLS VOD failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("encrypted.ts")).expect("read decrypted HLS artifact"),
        b"encrypted-one|encrypted-two|rotated-three|clear-four"
    );
    assert_eq!(
        server.hls_aes_first_key_attempts(),
        1,
        "concurrent segments sharing one key slot must share one key fetch"
    );
    assert_eq!(server.hls_aes_second_key_attempts(), 1);
}

#[tokio::test]
async fn paused_aes128_hls_vod_reopens_from_plaintext_checkpoint_and_refetches_keys() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create resumable AES-128 HLS directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("encrypted-resumed.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/index.m3u8", server.address()),
    }));
    server.hold_hls_aes_second_segment(true);

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit resumable AES-128 HLS VOD");
    let first_segment = destination
        .join(format!(".nxtools-hls-{}", task_id.as_uuid()))
        .join("00000000.segment");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !first_segment.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "first decrypted HLS segment did not become durable"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        std::fs::read(&first_segment).expect("read durable decrypted segment"),
        b"encrypted-one|"
    );
    engine.pause(task_id).expect("pause AES-128 HLS VOD");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for AES-128 HLS pause");
    server.hold_hls_aes_second_segment(false);
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(first_segment.exists());
    drop(engine);

    let reopened = DownloadEngine::open(&database).expect("reopen persistent download engine");
    reopened.resume(task_id).expect("resume AES-128 HLS VOD");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for resumed AES-128 HLS terminal state");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "resumed AES-128 HLS failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("encrypted-resumed.ts"))
            .expect("read resumed decrypted HLS artifact"),
        b"encrypted-one|encrypted-two|rotated-three|clear-four"
    );
    assert_eq!(
        server.hls_aes_first_key_attempts(),
        2,
        "keys stay memory-only and are refetched after process reopen"
    );
}

#[tokio::test]
async fn invalid_aes128_key_length_fails_without_committing_ciphertext() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create invalid AES-128 HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("invalid-key.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/bad-key.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit invalid-key AES-128 HLS VOD");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for invalid-key AES-128 HLS terminal state");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(!destination.join("invalid-key.ts").exists());
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    let retained_parts = if working.is_dir() {
        std::fs::read_dir(&working)
            .expect("read retained HLS checkpoint directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".part"))
            .count()
    } else {
        0
    };
    assert_eq!(
        retained_parts, 0,
        "failed key fetch retains no ciphertext parts"
    );
}

#[tokio::test]
async fn invalid_aes128_padding_fails_without_committing_ciphertext() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create invalid-padding HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("invalid-padding.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/bad-padding.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit invalid-padding AES-128 HLS VOD");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for invalid-padding AES-128 terminal state");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(!destination.join("invalid-padding.ts").exists());
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    let retains_no_parts = !working.is_dir()
        || std::fs::read_dir(&working)
            .expect("read retained HLS checkpoint directory")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().ends_with(".part"));
    assert!(
        retains_no_parts,
        "invalid padding retains neither plaintext nor ciphertext staging"
    );
}

#[tokio::test]
async fn pausing_during_aes128_key_fetch_leaves_no_segment_staging() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create paused-key HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("paused-key.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/index.m3u8", server.address()),
    }));
    server.hold_hls_aes_first_key(true);

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit key-blocked AES-128 HLS VOD");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while server.hls_aes_first_key_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "AES-128 key request did not reach the fixture"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine
        .pause(task_id)
        .expect("pause during AES-128 key fetch");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for key-fetch pause");
    server.hold_hls_aes_first_key(false);

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    assert!(
        std::fs::read_dir(working)
            .expect("read key-fetch checkpoint directory")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().ends_with(".part"))
    );
}

#[tokio::test]
async fn pausing_after_aes128_decryption_failure_cleans_retry_staging() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create retry-pause HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("retry-paused.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/bad-padding.m3u8", server.address()),
    }));
    server.hold_hls_invalid_padding_retry(true);

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit bad-padding AES-128 HLS VOD");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while server.hls_invalid_padding_attempts() < 2 {
        assert!(
            std::time::Instant::now() < deadline,
            "invalid-padding segment did not enter its second attempt"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine
        .pause(task_id)
        .expect("pause during AES-128 retry backoff");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for retry-backoff pause");
    server.hold_hls_invalid_padding_retry(false);

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    assert!(
        std::fs::read_dir(working)
            .expect("read retry checkpoint directory")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().ends_with(".part")),
        "retry pause retains neither plaintext nor ciphertext staging"
    );
}

#[tokio::test]
async fn cancelling_with_partial_aes128_ciphertext_removes_task_owned_staging() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create ciphertext-cancel HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("ciphertext-cancel.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/cancel.m3u8", server.address()),
    }));
    server.hold_hls_ciphertext_body(true);

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit cancellable AES-128 HLS VOD");
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    let ciphertext = working.join("00000000.encrypted.part");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if std::fs::metadata(&ciphertext).is_ok_and(|metadata| metadata.len() > 0) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "partial AES-128 ciphertext did not reach disk"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine
        .cancel(task_id)
        .expect("cancel partial AES-128 ciphertext transfer");
    let cancelled = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for AES-128 ciphertext cancellation");
    server.hold_hls_ciphertext_body(false);

    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert!(
        !working.exists(),
        "cancellation removes the HLS task directory"
    );
    assert!(!destination.join("ciphertext-cancel.ts").exists());
}

#[tokio::test]
async fn selected_hls_vod_bounds_parallel_segment_transfers_by_connection_limit() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create concurrent HLS engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("concurrent.ts".into());
    hls_request.connection_limit = Some(3);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/concurrent/index.m3u8", server.address()),
    }));
    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit concurrent HLS VOD");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for concurrent HLS VOD terminal state");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "concurrent HLS VOD failed: {:?}",
        completed.error
    );
    let observed = server.max_concurrent_range_transfers();
    assert!(
        observed >= 2,
        "expected HLS segment concurrency, saw {observed}"
    );
    assert!(observed <= 3, "HLS exceeded connection limit: {observed}");
    assert_eq!(
        std::fs::read(destination.join("concurrent.ts")).expect("read concurrent HLS artifact"),
        b"segment-a|segment-b|segment-c|segment-d"
    );
}

#[tokio::test]
async fn live_hls_attempt_keeps_its_proxy_snapshot_after_the_global_policy_changes() {
    let first_proxy = DeterministicHttpServer::start_concurrent();
    let next_proxy = DeterministicHttpServer::start();
    first_proxy.hold_hls_first_segment(true);
    let policy = NetworkPolicy::new(NetworkSettings {
        proxy: NetworkProxySettings {
            mode: NetworkProxyMode::ManualHttp,
            host: first_proxy.address().ip().to_string(),
            port: first_proxy.address().port(),
        },
    })
    .expect("first proxy policy");
    let engine = DownloadEngine::open_in_memory_with_network_policy(policy.clone())
        .expect("open proxied engine");
    let destination = tempfile::tempdir().expect("proxy snapshot destination");
    let mut hls_request = request(&first_proxy, destination.path().to_owned());
    hls_request.source_url = "http://origin.invalid/hls/master.m3u8".into();
    hls_request.file_name = Some("proxy-snapshot.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: "http://origin.invalid/hls/concurrent/index.m3u8".into(),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit proxied HLS task");
    let first_segment_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while first_proxy.hls_first_segment_attempts() == 0 {
        assert!(
            tokio::time::Instant::now() < first_segment_deadline,
            "the first proxy never received the live attempt"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    policy
        .update(NetworkSettings {
            proxy: NetworkProxySettings {
                mode: NetworkProxyMode::ManualHttp,
                host: next_proxy.address().ip().to_string(),
                port: next_proxy.address().port(),
            },
        })
        .expect("publish next proxy policy");
    first_proxy.hold_hls_first_segment(false);

    let terminal = engine
        .wait_for_terminal(task_id, Duration::from_secs(10))
        .await
        .expect("wait for proxied HLS terminal");
    assert_eq!(terminal.task.status(), DownloadStatus::Completed);
    assert!(
        first_proxy.total_requests() >= 5,
        "playlist and every segment must stay on the attempt's first proxy"
    );
    assert_eq!(
        next_proxy.total_requests(),
        0,
        "a settings update must not re-snapshot inside the active attempt"
    );
    assert_eq!(
        std::fs::read(destination.path().join("proxy-snapshot.ts"))
            .expect("read completed HLS artifact"),
        b"segment-a|segment-b|segment-c|segment-d"
    );
}

#[tokio::test]
async fn paused_hls_vod_resumes_after_engine_reopen_from_durable_segment_checkpoint() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create resumable HLS engine directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("resumed.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/concurrent/index.m3u8", server.address()),
    }));
    server.hold_hls_second_segment(true);
    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit resumable HLS VOD");
    let first_segment = destination
        .join(format!(".nxtools-hls-{}", task_id.as_uuid()))
        .join("00000000.segment");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !first_segment.exists() || server.hls_second_segment_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "first HLS segment did not reach its durable artifact"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause HLS VOD");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused HLS VOD");
    server.hold_hls_second_segment(false);
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        first_segment.exists(),
        "pause must preserve completed segments"
    );
    drop(engine);

    let reopened = DownloadEngine::open(&database).expect("reopen persistent download engine");
    let restored = reopened
        .task(task_id)
        .expect("read restored HLS task")
        .expect("restored HLS task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Paused);
    reopened.resume(task_id).expect("resume restored HLS VOD");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for resumed HLS VOD");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "resumed HLS VOD failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("resumed.ts")).expect("read resumed HLS artifact"),
        b"segment-a|segment-b|segment-c|segment-d"
    );
    assert_eq!(
        server.hls_first_segment_attempts(),
        1,
        "a durable completed HLS segment must not be requested again after reopen"
    );
}

#[tokio::test]
async fn hls_pause_interrupts_a_worker_waiting_for_task_bandwidth() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create rate-limited HLS directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination);
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("rate-limited.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.speed_limit_bytes_per_second = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/rate-limited/index.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit rate-limited HLS VOD");
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let task = engine
            .task(task_id)
            .expect("read rate-limited HLS task")
            .expect("rate-limited HLS task exists");
        if task.task.status() == DownloadStatus::Downloading {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "HLS task did not start"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    engine.pause(task_id).expect("pause rate-limited HLS VOD");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("bandwidth wait must be pause-responsive");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
}

#[tokio::test]
async fn reopened_hls_vod_rejects_an_incompatible_media_playlist_before_mixing_segments() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create mutable HLS engine directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("mutable.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/concurrent/index.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit mutable HLS VOD");
    let first_segment = destination
        .join(format!(".nxtools-hls-{}", task_id.as_uuid()))
        .join("00000000.segment");
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !first_segment.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "first mutable HLS segment did not complete"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause mutable HLS VOD");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for mutable HLS pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    drop(engine);
    server.advance_resource_revision();

    let reopened = DownloadEngine::open(&database).expect("reopen persistent download engine");
    reopened.resume(task_id).expect("resume mutable HLS VOD");
    let failed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for incompatible HLS terminal state");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(!destination.join("mutable.ts").exists());
    assert!(
        first_segment.exists(),
        "identity rejection preserves old progress"
    );
}

#[tokio::test]
async fn hls_cancel_cleanup_failure_is_terminal_and_preserves_checkpoint_ownership() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create HLS cleanup directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("cleanup.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/concurrent/index.m3u8", server.address()),
    }));
    server.hold_hls_first_segment(true);

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit cleanup HLS VOD");
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    let moved = destination.join(format!(".nxtools-hls-moved-{}", task_id.as_uuid()));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while server.hls_first_segment_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "HLS segment request did not reach the fixture"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(working.is_dir(), "HLS staging exists before segment I/O");
    std::fs::rename(&working, &moved).expect("move HLS staging out of cleanup path");
    std::fs::write(&working, b"cleanup obstruction").expect("obstruct HLS cleanup directory");
    engine.cancel(task_id).expect("request HLS cancellation");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for HLS cleanup failure");
    server.hold_hls_first_segment(false);

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(
        working.is_file(),
        "failed cleanup artifact remains observable"
    );
    assert!(
        engine.cancel(task_id).is_err(),
        "retrying cancellation must not claim success while HLS cleanup is still obstructed"
    );
    assert_eq!(
        engine
            .task(task_id)
            .expect("read cleanup-failed HLS task")
            .expect("cleanup-failed HLS task exists")
            .task
            .status(),
        DownloadStatus::Failed
    );
}

#[tokio::test]
async fn cancelling_a_paused_hls_task_removes_segments_and_durable_plan() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create paused HLS cancellation directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open persistent download engine");
    let mut hls_request = request(&server, destination.clone());
    hls_request.source_url = format!("http://{}/hls/master.m3u8", server.address());
    hls_request.file_name = Some("paused-cancel.ts".into());
    hls_request.connection_limit = Some(1);
    hls_request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/concurrent/index.m3u8", server.address()),
    }));

    let task_id = engine
        .submit(hls_request, RequestOrigin::Ui)
        .expect("submit paused cancellation HLS VOD");
    let working = destination.join(format!(".nxtools-hls-{}", task_id.as_uuid()));
    let first_segment = working.join("00000000.segment");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !first_segment.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "first HLS segment did not become durable"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine
        .pause(task_id)
        .expect("pause HLS task before cancellation");
    assert_eq!(
        engine
            .wait_for_terminal(task_id, Duration::from_secs(2))
            .await
            .expect("wait for paused HLS task")
            .task
            .status(),
        DownloadStatus::Paused
    );

    let cancel_correlation = uuid::Uuid::new_v4();
    let cancel_result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: cancel_correlation,
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Task {
            task_id,
            action: DownloadTaskAction::Cancel,
        },
    });
    assert_eq!(cancel_result.request_id, cancel_correlation);
    assert!(matches!(
        cancel_result.outcome,
        DownloadCommandOutcome::Accepted {
            task_id: Some(accepted_task_id)
        } if accepted_task_id == task_id
    ));
    assert_eq!(
        engine
            .task(task_id)
            .expect("read cancelled HLS task")
            .expect("cancelled HLS task exists")
            .task
            .status(),
        DownloadStatus::Cancelled
    );
    assert!(
        !working.exists(),
        "paused cancellation removes HLS segments"
    );
    let connection = rusqlite::Connection::open(&database).expect("open HLS cancellation database");
    let plan_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM download_hls_plans WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
            |row| row.get(0),
        )
        .expect("count retained HLS plans");
    assert_eq!(plan_count, 0, "paused cancellation clears the HLS plan");
    drop(connection);

    std::fs::create_dir_all(&working).expect("recreate orphaned HLS directory");
    std::fs::write(working.join("orphan.segment"), b"orphan").expect("write orphaned HLS segment");
    let remove_correlation = uuid::Uuid::new_v4();
    let remove_result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: remove_correlation,
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Task {
            task_id,
            action: DownloadTaskAction::Remove {
                delete_artifact: false,
            },
        },
    });
    assert_eq!(remove_result.request_id, remove_correlation);
    assert!(matches!(
        remove_result.outcome,
        DownloadCommandOutcome::Accepted {
            task_id: Some(accepted_task_id)
        } if accepted_task_id == task_id
    ));
    assert!(
        !working.exists(),
        "task removal cleans any remaining task-owned HLS directory"
    );
}

#[test]
fn completed_browser_blob_is_imported_once_through_the_engine_artifact_boundary() {
    let directory = tempfile::tempdir().expect("create Blob import directory");
    let database = directory.path().join("downloads.sqlite3");
    let staging = directory.path().join("browser-blob-staging");
    let destination = directory.path().join("downloads");
    std::fs::create_dir_all(&staging).expect("create Blob staging root");
    std::fs::create_dir_all(&destination).expect("create Blob destination");
    std::fs::write(destination.join("report.bin"), b"existing").expect("seed occupied Blob target");
    let blob_id = uuid::Uuid::new_v4();
    let staged_path = staging.join(format!("{blob_id}.blob"));
    std::fs::write(&staged_path, b"browser-blob").expect("stage Blob bytes");
    let handoff_id = uuid::Uuid::new_v4();
    let engine = DownloadEngine::open(&database).expect("open download engine");
    let command = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserImportBlob {
            handoff_id,
            blob: BrowserBlob {
                id: blob_id,
                file_name: "report.bin".into(),
                media_type: Some("application/octet-stream".into()),
                size: 12,
            },
            save_directory: destination.clone(),
            file_name: "report.bin".into(),
            category_id: None,
            save_directory_explicit: false,
        },
    };

    let accepted = engine.handle_command(command.clone());
    let task_id = match accepted.outcome {
        DownloadCommandOutcome::BrowserAccepted {
            task_id,
            duplicate: false,
        } => task_id,
        outcome => panic!("unexpected Blob import outcome: {outcome:?}"),
    };
    let task = engine
        .task(task_id)
        .expect("read imported task")
        .expect("imported task exists");
    assert_eq!(task.task.status(), DownloadStatus::Completed);
    assert_eq!(task.downloaded_bytes, 12);
    assert_eq!(
        std::fs::read(destination.join("report (1).bin")).expect("read imported Blob"),
        b"browser-blob"
    );
    assert_eq!(
        std::fs::read(destination.join("report.bin")).expect("read preserved existing Blob target"),
        b"existing"
    );
    assert!(!staged_path.exists());

    let duplicate = engine.handle_command(command);
    assert!(matches!(
        duplicate.outcome,
        DownloadCommandOutcome::BrowserAccepted {
            task_id: duplicate_task_id,
            duplicate: true,
        } if duplicate_task_id == task_id
    ));

    let conflict = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserImportBlob {
            handoff_id,
            blob: BrowserBlob {
                id: uuid::Uuid::new_v4(),
                file_name: "different.bin".into(),
                media_type: Some("application/octet-stream".into()),
                size: 1,
            },
            save_directory: destination.clone(),
            file_name: "different.bin".into(),
            category_id: None,
            save_directory_explicit: false,
        },
    });
    assert!(matches!(
        conflict.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::InvalidRequest
                && fault.message.contains("conflicts with a previously accepted request")
    ));
}

#[test]
fn completed_browser_blob_reaches_the_private_dialog_without_a_network_probe() {
    let directory = tempfile::tempdir().expect("create Blob dialog directory");
    let database = directory.path().join("downloads.sqlite3");
    let staging = directory.path().join("browser-blob-staging");
    std::fs::create_dir_all(&staging).expect("create Blob staging root");
    let blob_id = uuid::Uuid::new_v4();
    let handoff_id = uuid::Uuid::new_v4();
    let staged_path = staging.join(format!("{blob_id}.blob"));
    std::fs::write(&staged_path, b"dialog-blob").expect("stage dialog Blob");
    let engine = DownloadEngine::open(database).expect("open download engine");
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach GPUI dialog consumer");
    let command_engine = engine.clone();
    let save_directory = directory.path().join("downloads");
    let expected_directory = save_directory.clone();
    let command = std::thread::spawn(move || {
        command_engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command: DownloadCommand::BrowserOpenBlobDialog {
                handoff_id,
                blob: BrowserBlob {
                    id: blob_id,
                    file_name: "dialog.bin".into(),
                    media_type: Some("application/octet-stream".into()),
                    size: 11,
                },
                save_directory,
            },
        })
    });
    let dialog = dialogs.recv_blocking().expect("receive Blob dialog");
    match dialog.source {
        nxtools_download::BrowserDownloadDialogSource::Blob {
            blob,
            save_directory,
        } => {
            assert_eq!(blob.id, blob_id);
            assert_eq!(blob.size, 11);
            assert_eq!(save_directory, expected_directory);
        }
        source => panic!("unexpected dialog source: {source:?}"),
    }
    dialog.completion.send(Ok(())).expect("confirm dialog open");
    assert!(matches!(
        command.join().expect("join Blob dialog command").outcome,
        DownloadCommandOutcome::BrowserDialogAccepted {
            duplicate: false,
            ..
        }
    ));
    let replayed = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserOpenBlobDialog {
            handoff_id,
            blob: BrowserBlob {
                id: uuid::Uuid::new_v4(),
                file_name: "replayed.bin".into(),
                media_type: Some("application/octet-stream".into()),
                size: 1,
            },
            save_directory: expected_directory,
        },
    });
    assert!(matches!(
        replayed.outcome,
        DownloadCommandOutcome::BrowserDialogAccepted {
            duplicate: true,
            ..
        }
    ));
    let discarded = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::BrowserDiscardBlob {
            handoff_id,
            blob_id,
        },
    });
    assert!(matches!(
        discarded.outcome,
        DownloadCommandOutcome::Accepted { task_id: None }
    ));
    assert!(!staged_path.exists());
}

#[tokio::test]
async fn delayed_submission_stays_dormant_until_its_due_time_then_enters_the_queue() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let due_at_unix_ms = 9_000_000_000_000_i64;
    let mut delayed = request(&server, directory.path().to_path_buf());
    delayed.scheduled_at_unix_ms = Some(due_at_unix_ms);

    let task_id = engine
        .submit(delayed, RequestOrigin::Ui)
        .expect("accept delayed submission");
    let before_due = engine
        .task(task_id)
        .expect("read delayed task")
        .expect("delayed task exists");
    assert_eq!(before_due.task.status(), DownloadStatus::Created);
    assert!(!before_due.queue_eligible);
    assert_eq!(
        before_due.task.request().scheduled_at_unix_ms,
        Some(due_at_unix_ms)
    );

    engine
        .process_due_schedules_at(due_at_unix_ms)
        .expect("release task at due time");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let after_due = engine
        .task(task_id)
        .expect("read released task")
        .expect("released task exists");
    assert_ne!(after_due.task.status(), DownloadStatus::Created);
    assert!(
        !after_due.queue_eligible,
        "queue admission consumes eligibility once a transfer slot is assigned"
    );
    assert_eq!(after_due.task.request().scheduled_at_unix_ms, None);
}

#[tokio::test]
async fn delayed_submission_survives_restart_and_is_released_when_it_becomes_due() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.db");
    let due_at_unix_ms = 9_000_000_000_000_i64;
    let task_id = {
        let engine = DownloadEngine::open(&database).expect("open first engine");
        let mut delayed = request(&server, directory.path().join("artifact"));
        delayed.scheduled_at_unix_ms = Some(due_at_unix_ms);
        engine
            .submit(delayed, RequestOrigin::Ui)
            .expect("accept delayed submission")
    };

    let reopened = DownloadEngine::open(&database).expect("reopen engine");
    let restored = reopened
        .task(task_id)
        .expect("read restored task")
        .expect("restored task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Created);
    assert!(!restored.queue_eligible);
    assert_eq!(
        restored.task.request().scheduled_at_unix_ms,
        Some(due_at_unix_ms)
    );

    reopened
        .process_due_schedules_at(due_at_unix_ms)
        .expect("release restored delayed submission");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let released = reopened
        .task(task_id)
        .expect("read released restored task")
        .expect("released task exists");
    assert_ne!(released.task.status(), DownloadStatus::Created);
    assert_eq!(released.task.request().scheduled_at_unix_ms, None);
}

#[test]
fn daily_schedule_window_handles_normal_and_cross_midnight_selected_days() {
    let normal = DownloadDailySchedule {
        days: vec![DownloadWeekday::Monday],
        start_minute: 9 * 60,
        stop_minute: 17 * 60,
        enabled_start: true,
        enabled_stop: true,
    };
    assert!(daily_schedule_window_contains(
        &normal,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 9 * 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &normal,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 17 * 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &normal,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Tuesday,
            minute_of_day: 9 * 60,
        }
    ));

    let overnight = DownloadDailySchedule {
        days: vec![DownloadWeekday::Friday],
        start_minute: 22 * 60,
        stop_minute: 2 * 60,
        enabled_start: true,
        enabled_stop: true,
    };
    assert!(daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Friday,
            minute_of_day: 23 * 60,
        }
    ));
    assert!(daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Saturday,
            minute_of_day: 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Saturday,
            minute_of_day: 2 * 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Saturday,
            minute_of_day: 3 * 60,
        }
    ));
}

#[test]
fn browser_handoff_is_idempotent_across_native_host_and_engine_restarts() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.db");
    let handoff_id = uuid::Uuid::new_v4();
    let first_task_id = {
        let engine = DownloadEngine::open(&database).expect("open first engine");
        let first = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command: DownloadCommand::BrowserSubmit {
                handoff_id,
                request: request(&server, directory.path().join("artifacts")),
                mode: DownloadSubmitMode::QueueOnly,
            },
        });
        let DownloadCommandOutcome::BrowserAccepted {
            task_id,
            duplicate: false,
        } = first.outcome
        else {
            panic!("first browser handoff was not accepted");
        };

        let duplicate = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command: DownloadCommand::BrowserSubmit {
                handoff_id,
                request: request(&server, directory.path().join("artifacts")),
                mode: DownloadSubmitMode::QueueOnly,
            },
        });
        assert!(matches!(
            duplicate.outcome,
            DownloadCommandOutcome::BrowserAccepted {
                task_id: duplicate_task_id,
                duplicate: true,
            } if duplicate_task_id == task_id
        ));
        task_id
    };

    let reopened = DownloadEngine::open(&database).expect("reopen engine");
    let after_restart = reopened.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserSubmit {
            handoff_id,
            request: request(&server, directory.path().join("artifacts")),
            mode: DownloadSubmitMode::QueueOnly,
        },
    });
    assert!(matches!(
        after_restart.outcome,
        DownloadCommandOutcome::BrowserAccepted {
            task_id,
            duplicate: true,
        } if task_id == first_task_id
    ));
}

#[test]
fn legacy_browser_handoff_replay_is_backfilled_at_the_engine_boundary() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create legacy handoff directory");
    let database = directory.path().join("downloads.sqlite3");
    let handoff_id = uuid::Uuid::new_v4();
    let original_request = request(&server, directory.path().join("artifacts"));

    {
        let engine = DownloadEngine::open(&database).expect("open first engine");
        let accepted = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command: DownloadCommand::BrowserSubmit {
                handoff_id,
                request: original_request.clone(),
                mode: DownloadSubmitMode::QueueOnly,
            },
        });
        assert!(matches!(
            accepted.outcome,
            DownloadCommandOutcome::BrowserAccepted {
                duplicate: false,
                ..
            }
        ));
    }

    let connection = rusqlite::Connection::open(&database).expect("open legacy handoff database");
    assert_eq!(
            connection
                .execute(
                    "UPDATE download_browser_handoffs SET request_fingerprint = '' WHERE handoff_id = ?1",
                    [handoff_id.to_string()],
                )
                .expect("clear legacy handoff fingerprint"),
            1
        );
    drop(connection);

    let engine = DownloadEngine::open(&database).expect("reopen legacy handoff engine");
    let replay = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserSubmit {
            handoff_id,
            request: original_request,
            mode: DownloadSubmitMode::QueueOnly,
        },
    });
    let original_task_id = match replay.outcome {
        DownloadCommandOutcome::BrowserAccepted {
            task_id,
            duplicate: true,
        } => task_id,
        outcome => panic!("legacy handoff replay was not a duplicate: {outcome:?}"),
    };

    let mut conflicting_request = request(&server, directory.path().join("other"));
    conflicting_request.source_url = "http://127.0.0.1:9/must-not-be-probed".into();
    let conflict = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserSubmit {
            handoff_id,
            request: conflicting_request,
            mode: DownloadSubmitMode::QueueOnly,
        },
    });
    assert!(matches!(
        conflict.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::InvalidRequest
                && fault.message.contains("conflicts with a previously accepted request")
    ));
    let tasks = engine.tasks().expect("read tasks after legacy replay");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task.id(), original_task_id);
}

#[test]
fn browser_handoff_rejects_a_different_request_body_without_creating_a_second_task() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let handoff_id = uuid::Uuid::new_v4();
    let accepted = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserSubmit {
            handoff_id,
            request: request(&server, directory.path().join("first")),
            mode: DownloadSubmitMode::QueueOnly,
        },
    });
    assert!(matches!(
        accepted.outcome,
        DownloadCommandOutcome::BrowserAccepted {
            duplicate: false,
            ..
        }
    ));

    let mut conflicting_request = request(&server, directory.path().join("second"));
    conflicting_request.source_url = "http://127.0.0.1:9/must-not-be-probed".into();
    let conflict = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserSubmit {
            handoff_id,
            request: conflicting_request,
            mode: DownloadSubmitMode::QueueOnly,
        },
    });
    assert!(matches!(
        conflict.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::InvalidRequest
                && fault.message.contains("conflicts with a previously accepted request")
    ));
    assert_eq!(engine.tasks().expect("read tasks after conflict").len(), 1);
}

#[test]
fn idempotent_submit_replays_across_engine_restart_and_rejects_a_conflict() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create idempotent submit directory");
    let database = directory.path().join("downloads.sqlite3");
    let key = uuid::Uuid::new_v4();
    let original_request = request(&server, directory.path().join("artifacts"));
    let first_task_id = {
        let engine = DownloadEngine::open(&database).expect("open first idempotent engine");
        let result = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Cli,
            command: DownloadCommand::Submit {
                request: original_request.clone(),
                mode: DownloadSubmitMode::QueueOnly,
                idempotency_key: Some(key),
            },
        });
        match result.outcome {
            DownloadCommandOutcome::IdempotentAccepted {
                task_id,
                duplicate: false,
            } => task_id,
            outcome => panic!("unexpected first idempotent outcome: {outcome:?}"),
        }
    };

    let reopened = DownloadEngine::open(&database).expect("reopen idempotent engine");
    let replay = reopened.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Submit {
            request: original_request,
            mode: DownloadSubmitMode::QueueOnly,
            idempotency_key: Some(key),
        },
    });
    assert!(matches!(
        replay.outcome,
        DownloadCommandOutcome::IdempotentAccepted {
            task_id,
            duplicate: true,
        } if task_id == first_task_id
    ));

    let mut conflicting_request = request(&server, directory.path().join("other"));
    conflicting_request.source_url = "http://127.0.0.1:9/must-not-be-probed".into();
    let conflict = reopened.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Submit {
            request: conflicting_request,
            mode: DownloadSubmitMode::QueueOnly,
            idempotency_key: Some(key),
        },
    });
    assert!(matches!(
        conflict.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::InvalidRequest
                && fault.message.contains("conflicts with a previously accepted request")
    ));
    assert_eq!(reopened.tasks().expect("read idempotent tasks").len(), 1);
}

#[test]
fn browser_batch_submission_creates_each_task_once_and_replays_by_handoff_identity() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut first_request = request(&server, directory.path().join("artifacts"));
    first_request.file_name = Some("first.bin".into());
    let mut second_request = first_request.clone();
    second_request.file_name = Some("second.bin".into());
    let submissions = vec![
        BrowserDownloadSubmission {
            handoff_id: uuid::Uuid::new_v4(),
            request: first_request,
        },
        BrowserDownloadSubmission {
            handoff_id: uuid::Uuid::new_v4(),
            request: second_request,
        },
    ];
    let command = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: submissions.clone(),
        },
    };

    let first = engine.handle_command(command.clone());
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = first.outcome else {
        panic!("browser batch was not accepted");
    };
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| !result.duplicate));
    assert_eq!(engine.tasks().expect("read tasks").len(), 2);

    let replay = engine.handle_command(command);
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = replay.outcome else {
        panic!("browser batch replay was not accepted");
    };
    assert_eq!(
        results
            .iter()
            .map(|result| result.handoff_id)
            .collect::<Vec<_>>(),
        submissions
            .iter()
            .map(|submission| submission.handoff_id)
            .collect::<Vec<_>>()
    );
    assert!(results.iter().all(|result| result.duplicate));
    assert_eq!(engine.tasks().expect("read tasks after replay").len(), 2);
}
#[test]
fn browser_batch_completion_membership_is_runtime_only_and_excludes_duplicate_handoffs() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create batch directory");
    let engine = DownloadEngine::open_in_memory().expect("open batch engine");
    let mut first = request(&server, directory.path().join("artifacts"));
    first.file_name = Some("first.bin".into());
    let mut second = first.clone();
    second.file_name = Some("second.bin".into());
    let handoff = uuid::Uuid::new_v4();
    let initial = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: vec![BrowserDownloadSubmission {
                handoff_id: handoff,
                request: first.clone(),
            }],
        },
    });
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = initial.outcome else {
        panic!("initial handoff rejected");
    };
    let original = results[0].task_id.expect("initial task id");
    let correlation = uuid::Uuid::new_v4();
    let second_batch = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: correlation,
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: vec![
                BrowserDownloadSubmission {
                    handoff_id: handoff,
                    request: first,
                },
                BrowserDownloadSubmission {
                    handoff_id: uuid::Uuid::new_v4(),
                    request: second,
                },
            ],
        },
    });
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = second_batch.outcome else {
        panic!("batch rejected");
    };
    assert!(results[0].duplicate);
    assert_eq!(results[0].task_id, Some(original));
    assert!(!results[1].duplicate);
    let snapshot = format!(
        "{:?}",
        engine.completion_snapshot().expect("completion snapshot")
    );
    assert!(
        snapshot.contains(&correlation.to_string()),
        "accepted batch identity is missing"
    );
    assert_eq!(
        snapshot.matches(&correlation.to_string()).count(),
        1,
        "duplicate joined batch"
    );
}

#[test]
fn browser_batch_rejects_an_empty_payload_as_an_invalid_request() {
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: Vec::new(),
        },
    });

    assert!(matches!(
        result.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::InvalidRequest
                && !fault.retryable
    ));
}

#[test]
fn browser_dialog_requires_an_attached_gpui_consumer_and_suppresses_duplicates() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let handoff_id = uuid::Uuid::new_v4();
    let request_id = uuid::Uuid::new_v4();
    let download_request = request(&server, directory.path().join("artifacts"));

    let unavailable = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id,
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserOpenDialog {
            handoff_id,
            request: download_request.clone(),
        },
    });
    assert!(matches!(
        unavailable.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::Unavailable && fault.retryable
    ));
    assert!(engine.tasks().expect("read tasks").is_empty());

    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach GPUI browser-dialog consumer");
    let command_engine = engine.clone();
    let command_request = download_request.clone();
    let accepted = std::thread::spawn(move || {
        command_engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id,
            origin: RequestOrigin::Browser,
            command: DownloadCommand::BrowserOpenDialog {
                handoff_id,
                request: command_request,
            },
        })
    });
    let dialog = dialogs
        .recv_blocking()
        .expect("receive browser dialog request");
    assert_eq!(dialog.correlation_id, request_id);
    assert_eq!(dialog.handoff_id, handoff_id);
    assert_eq!(
        dialog.source,
        nxtools_download::BrowserDownloadDialogSource::Download(Box::new(download_request.clone()))
    );
    dialog
        .completion
        .send(Ok(()))
        .expect("confirm GPUI window creation");
    let accepted = accepted.join().expect("browser dialog command completed");
    assert!(matches!(
        accepted.outcome,
        DownloadCommandOutcome::BrowserDialogAccepted {
            handoff_id: accepted_handoff_id,
            duplicate: false,
        } if accepted_handoff_id == handoff_id
    ));
    assert!(engine.tasks().expect("read tasks").is_empty());

    let duplicate = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserOpenDialog {
            handoff_id,
            request: download_request,
        },
    });
    assert!(matches!(
        duplicate.outcome,
        DownloadCommandOutcome::BrowserDialogAccepted {
            handoff_id: accepted_handoff_id,
            duplicate: true,
        } if accepted_handoff_id == handoff_id
    ));
    assert!(dialogs.try_recv().is_err());
}

#[test]
fn browser_dialog_window_creation_failure_is_terminal() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach GPUI browser-dialog consumer");
    let command_engine = engine.clone();
    let command = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserOpenDialog {
            handoff_id: uuid::Uuid::new_v4(),
            request: request(&server, directory.path().join("artifacts")),
        },
    };
    let result = std::thread::spawn(move || command_engine.handle_command(command));
    dialogs
        .recv_blocking()
        .expect("receive browser dialog request")
        .completion
        .send(Err("window creation failed".into()))
        .expect("reject GPUI window creation");
    let result = result.join().expect("browser dialog command completed");

    assert!(matches!(
        result.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == nxtools_download::DownloadFaultCode::Unavailable && !fault.retryable
    ));
}

#[tokio::test]
async fn daily_queue_window_releases_queue_only_tasks_and_stops_after_the_window() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let task_id = engine
        .enqueue(
            request(&server, directory.path().to_path_buf()),
            RequestOrigin::Ui,
        )
        .expect("queue task without immediate admission");
    engine
        .set_queue_schedule(
            "default",
            Some(DownloadSchedule {
                daily: DownloadDailySchedule {
                    days: vec![DownloadWeekday::Monday],
                    start_minute: 9 * 60,
                    stop_minute: 17 * 60,
                    enabled_start: true,
                    enabled_stop: true,
                },
            }),
        )
        .expect("save daily queue schedule");

    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 18 * 60,
        })
        .expect("close queue outside the daily window");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 9 * 60,
        })
        .expect("open queue at start boundary");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_ne!(
        engine
            .task(task_id)
            .expect("read scheduled task")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Created
    );

    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 17 * 60,
        })
        .expect("close queue at the stop boundary");
    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Snapshot {
            query: Default::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
        panic!("snapshot expected");
    };
    assert!(!snapshot.queues[0].is_running);
}

#[tokio::test]
async fn manual_pause_is_not_overridden_by_the_next_daily_start() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut queued = request(&server, directory.path().to_path_buf());
    queued.speed_limit_bytes_per_second = Some(1);
    let task_id = engine
        .enqueue(queued, RequestOrigin::Ui)
        .expect("queue task");
    // set_queue_schedule reconciles against the real local clock. Pick a schedule day that cannot
    // be today so the deterministic moments below own the first start/stop transition.
    let schedule_day = match Local::now().weekday() {
        Weekday::Mon => DownloadWeekday::Tuesday,
        _ => DownloadWeekday::Monday,
    };
    engine
        .set_queue_schedule(
            "default",
            Some(DownloadSchedule {
                daily: DownloadDailySchedule {
                    days: vec![schedule_day],
                    start_minute: 9 * 60,
                    stop_minute: 17 * 60,
                    enabled_start: true,
                    enabled_stop: true,
                },
            }),
        )
        .expect("save schedule");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: schedule_day,
            minute_of_day: 18 * 60,
        })
        .expect("close queue");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: schedule_day,
            minute_of_day: 9 * 60,
        })
        .expect("start queue");
    tokio::time::sleep(Duration::from_millis(50)).await;
    engine.pause(task_id).expect("manually pause task");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        engine
            .task(task_id)
            .expect("read manually paused task")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Paused
    );

    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: schedule_day,
            minute_of_day: 18 * 60,
        })
        .expect("close next window");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: schedule_day,
            minute_of_day: 9 * 60,
        })
        .expect("open next window");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        engine
            .task(task_id)
            .expect("read task after next start")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Paused
    );
}

#[tokio::test]
async fn queue_move_and_batch_commands_preserve_order_and_report_each_target() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let changed = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::SetMaxConcurrent {
                queue_id: "default".into(),
                max_concurrent_tasks: 1,
            },
        },
    });
    assert!(matches!(
        changed.outcome,
        DownloadCommandOutcome::QueueChanged { .. }
    ));
    let mut first_request = request(&server, directory.path().join("first"));
    first_request.speed_limit_bytes_per_second = Some(1);
    let first = engine
        .submit(first_request, RequestOrigin::Ui)
        .expect("submit first");
    let second = engine
        .submit(
            request(&server, directory.path().join("second")),
            RequestOrigin::Ui,
        )
        .expect("submit second");
    let third = engine
        .submit(
            request(&server, directory.path().join("third")),
            RequestOrigin::Ui,
        )
        .expect("submit third");

    let moved = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::Move {
                queue_id: "default".into(),
                task_ids: vec![third, second],
                placement: DownloadQueuePlacement::Top,
            },
        },
    });
    assert!(matches!(
        moved.outcome,
        DownloadCommandOutcome::QueueChanged { .. }
    ));
    assert_eq!(
        engine.queue_order("default").expect("read queue order"),
        vec![second, third, first],
        "selected tasks keep their existing relative order"
    );

    let missing = nxtools_download::DownloadTaskId::new();
    let batch = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Batch {
            task_ids: vec![first, missing, second],
            action: DownloadBatchAction::Cancel,
        },
    });
    let DownloadCommandOutcome::Batch { results } = batch.outcome else {
        panic!("batch outcome expected");
    };
    assert_eq!(results.len(), 3);
    assert!(results[0].fault.is_none());
    assert!(results[1].fault.is_some());
    assert!(results[2].fault.is_none());
    engine.cancel(third).expect("cancel third");
    engine
        .wait_for_terminal(first, Duration::from_secs(2))
        .await
        .expect("wait for first cancellation");
    engine
        .wait_for_terminal(third, Duration::from_secs(2))
        .await
        .expect("wait for third cancellation");
}

#[tokio::test]
async fn queue_concurrency_admits_only_one_task_and_refills_released_slot() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let changed = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::SetMaxConcurrent {
                queue_id: "default".into(),
                max_concurrent_tasks: 1,
            },
        },
    });
    assert!(matches!(
        changed.outcome,
        DownloadCommandOutcome::QueueChanged { .. }
    ));

    let mut first_request = request(&server, directory.path().join("first"));
    first_request.speed_limit_bytes_per_second = Some(1);
    let mut second_request = request(&server, directory.path().join("second"));
    second_request.speed_limit_bytes_per_second = Some(1);
    let first = engine
        .submit(first_request, RequestOrigin::Ui)
        .expect("submit first");
    let second = engine
        .submit(second_request, RequestOrigin::Ui)
        .expect("submit second");

    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_ne!(
        engine
            .task(first)
            .expect("read first")
            .expect("first exists")
            .task
            .status(),
        DownloadStatus::Created
    );
    assert_eq!(
        engine
            .task(second)
            .expect("read second")
            .expect("second exists")
            .task
            .status(),
        DownloadStatus::Created
    );
    engine.pause(second).expect("pause pending task");
    assert_eq!(
        engine
            .task(second)
            .expect("read paused pending task")
            .expect("second exists")
            .task
            .status(),
        DownloadStatus::Paused
    );
    assert!(
        !engine
            .task(second)
            .expect("read paused pending task eligibility")
            .expect("second exists")
            .queue_eligible
    );
    engine
        .resume(second)
        .expect("mark pending task eligible again");
    assert!(
        engine
            .task(second)
            .expect("read resumed pending task eligibility")
            .expect("second exists")
            .queue_eligible
    );
    engine
        .pause(second)
        .expect("pause resumed task before it receives a slot");
    engine.cancel(first).expect("cancel first");
    engine
        .wait_for_terminal(first, Duration::from_secs(2))
        .await
        .expect("wait for first cancellation");
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(
        engine
            .task(second)
            .expect("read paused task after slot release")
            .expect("second exists")
            .task
            .status(),
        DownloadStatus::Paused,
        "a pending pause must remove the task from automatic admission"
    );
    engine.resume(second).expect("resume pending task");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let status = engine
            .task(second)
            .expect("read second")
            .expect("second exists")
            .task
            .status();
        if status != DownloadStatus::Paused {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "second task was not admitted after the first released its slot"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !engine
            .task(second)
            .expect("read admitted task eligibility")
            .expect("second exists")
            .queue_eligible
    );
    engine.cancel(second).expect("cancel second");
    engine
        .wait_for_terminal(second, Duration::from_secs(2))
        .await
        .expect("wait for second cancellation");
}

#[tokio::test]
async fn reopening_engine_admits_a_durable_eligible_pending_task() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.sqlite3");
    let queued = DownloadTask::new(
        request(&server, directory.path().join("artifacts")),
        RequestOrigin::Ui,
        1_000,
    );
    let task_id = queued.id();
    {
        let store = SqliteDownloadStore::open(&database).expect("open store");
        store.save_task(&queued).expect("save queued task");
        store
            .append_to_queue("default", task_id, true)
            .expect("mark task eligible");
    }

    let engine = DownloadEngine::open(&database).expect("reopen engine");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for startup admission");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
}

#[tokio::test]
async fn queue_concurrency_is_scoped_per_queue() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.sqlite3");
    let mut default_request = request(&server, directory.path().join("default"));
    default_request.speed_limit_bytes_per_second = Some(1);
    let mut secondary_request = request(&server, directory.path().join("secondary"));
    secondary_request.speed_limit_bytes_per_second = Some(1);
    let default_task = DownloadTask::new(default_request, RequestOrigin::Ui, 1_000);
    let secondary_task = DownloadTask::new(secondary_request, RequestOrigin::Ui, 2_000);
    {
        let store = SqliteDownloadStore::open(&database).expect("open store");
        store.save_task(&default_task).expect("save default task");
        store
            .append_to_queue("default", default_task.id(), true)
            .expect("queue default task");
        store
            .save_task(&secondary_task)
            .expect("save secondary task");
        store
            .append_to_queue("secondary", secondary_task.id(), true)
            .expect("queue secondary task");
        store
            .set_queue_max_concurrent("secondary", 1)
            .expect("set secondary concurrency");
    }

    let engine = DownloadEngine::open(&database).expect("open engine");
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_ne!(
        engine
            .task(secondary_task.id())
            .expect("read secondary")
            .expect("secondary exists")
            .task
            .status(),
        DownloadStatus::Created,
        "activity in the default queue must not consume a secondary queue slot"
    );
    engine
        .cancel(default_task.id())
        .expect("cancel default task");
    engine
        .cancel(secondary_task.id())
        .expect("cancel secondary task");
    engine
        .wait_for_terminal(default_task.id(), Duration::from_secs(2))
        .await
        .expect("wait for default cancellation");
    engine
        .wait_for_terminal(secondary_task.id(), Duration::from_secs(2))
        .await
        .expect("wait for secondary cancellation");
}

#[test]
fn crash_helper_process() {
    if std::env::var_os("NXTOOLS_DOWNLOAD_CRASH_HELPER").is_none() {
        return;
    }
    let database_path =
        PathBuf::from(std::env::var_os("NXTOOLS_DOWNLOAD_CRASH_DB").expect("crash database path"));
    let destination = PathBuf::from(
        std::env::var_os("NXTOOLS_DOWNLOAD_CRASH_DESTINATION").expect("crash destination path"),
    );
    let source_url = std::env::var("NXTOOLS_DOWNLOAD_CRASH_URL").expect("crash fixture source URL");
    let engine = DownloadEngine::open(&database_path).expect("open crash-helper engine");
    let task_id = engine
        .submit(
            DownloadRequest {
                source_url,
                save_directory: destination,
                save_directory_explicit: false,
                file_name: Some("forced-exit.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Internal,
        )
        .expect("submit crash-helper task");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let task = engine
            .task(task_id)
            .expect("read crash-helper task")
            .expect("crash-helper task exists");
        if task.downloaded_bytes > 0
            && task
                .temporary_path
                .as_ref()
                .is_some_and(|path| path.exists())
        {
            std::process::exit(86);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "crash-helper transfer did not start"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[tokio::test]
async fn one_engine_task_downloads_persists_and_reopens_as_completed() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");

    let task_id = engine
        .submit(request(&server, destination.clone()), RequestOrigin::Ui)
        .expect("submit task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for terminal task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "download failed: {:?}",
        completed.error
    );
    assert_eq!(completed.downloaded_bytes, 16);
    assert_eq!(completed.total_bytes, Some(16));
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read completed download"),
        b"0123456789abcdef"
    );
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("load task")
        .expect("persisted task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Completed);
    assert_eq!(restored.downloaded_bytes, 16);
    assert_eq!(restored.total_bytes, Some(16));
}

#[tokio::test]
async fn paused_task_keeps_its_identity_and_can_restart_to_completion() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut slow_request = request(&server, destination.clone());
    slow_request.source_url = format!("http://{}/stall", server.address());

    let task_id = engine
        .submit(slow_request, RequestOrigin::Ui)
        .expect("submit slow task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("request pause");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused task");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0);
    assert!(!destination.join("engine-fixture.bin").exists());

    engine.resume(task_id).expect("resume paused task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for resumed task");
    assert_eq!(completed.task.id(), task_id);
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(completed.downloaded_bytes, 16);
}

#[tokio::test]
async fn ranged_task_resumes_persisted_bytes_after_the_engine_is_reopened() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut ranged_request = request(&server, destination.clone());
    ranged_request.source_url = format!(
        "http://{}/range-resume-required?checkpoint=1",
        server.address()
    );
    ranged_request.speed_limit_bytes_per_second = Some(64 * 1024);

    let task_id = engine
        .submit(ranged_request, RequestOrigin::Ui)
        .expect("submit ranged task");
    wait_for_partial_progress(&engine, task_id, concurrent_fixture_bytes().len() as u64).await;
    engine.pause(task_id).expect("request pause");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused task");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        paused.downloaded_bytes > 0
            && paused.downloaded_bytes < concurrent_fixture_bytes().len() as u64
    );
    let checkpoint = paused.downloaded_bytes;
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("read task")
        .expect("persisted task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Paused);
    assert_eq!(restored.downloaded_bytes, checkpoint);

    reopened
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before resume");
    reopened.resume(task_id).expect("resume persisted task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for resumed task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(
        completed.downloaded_bytes,
        concurrent_fixture_bytes().len() as u64
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read resumed artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn numbered_retry_keeps_the_original_segment_checkpoint_artifact() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create checkpoint directory");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database).expect("open checkpoint engine");
    let mut task_request = request(&server, destination.clone());
    task_request.source_url = format!(
        "http://{}/range-resume-required?checkpoint=1",
        server.address()
    );
    task_request.speed_limit_bytes_per_second = Some(64 * 1024);
    let task_id = engine
        .submit(task_request, RequestOrigin::Ui)
        .expect("start ranged transfer");
    wait_for_partial_progress(&engine, task_id, concurrent_fixture_bytes().len() as u64).await;
    engine.pause(task_id).expect("pause checkpoint");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    let original_temp = paused.temporary_path.expect("durable partial file");
    let checkpoint = paused.downloaded_bytes;
    assert!(checkpoint > 0 && checkpoint < concurrent_fixture_bytes().len() as u64);
    std::fs::write(destination.join("engine-fixture.bin"), b"foreign bytes")
        .expect("occupy old destination");
    drop(engine);
    let engine = DownloadEngine::open(&database).expect("restore checkpoint engine");
    engine.resume(task_id).expect("retry original target");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for conflict");
    assert_eq!(
        failed.error.as_ref().map(|error| error.code),
        Some(nxtools_download::DownloadFaultCode::DestinationExists)
    );
    let command_id = uuid::Uuid::new_v4();
    engine
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before retry");
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: command_id,
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Task {
            task_id,
            action: DownloadTaskAction::RetryWithAutoRename,
        },
    });
    assert_eq!(result.request_id, command_id);
    assert!(
        matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == task_id)
    );
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("resume original checkpoint");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        completed.task.request().file_name.as_deref(),
        Some("engine-fixture (1).bin")
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture (1).bin")).expect("numbered bytes"),
        concurrent_fixture_bytes()
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("foreign file"),
        b"foreign bytes"
    );
    assert!(!original_temp.exists());
}

#[tokio::test]
async fn range_ignored_after_probe_restarts_cleanly_as_a_single_stream() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-probe-only", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for fallback task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "range fallback failed after {} durable bytes: {:?}",
        completed.downloaded_bytes,
        completed.error
    );
    assert_eq!(completed.downloaded_bytes, 16);
    assert_eq!(
        server.fallback_stream_attempts(),
        2,
        "one transient fallback failure must schedule exactly one retry"
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read fallback artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn single_stream_retry_budget_stops_after_three_attempts() {
    let server = DeterministicHttpServer::start_concurrent();
    server.set_fallback_stream_failures(u64::MAX);
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-probe-only-unavailable", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit task");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for exhausted fallback task");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(
        failed.error.as_ref().is_some_and(|fault| fault.retryable),
        "exhausted transient failure must remain retryable: {:?}",
        failed.error
    );
    assert!(
        (2..=3).contains(&server.fallback_stream_attempts()),
        "the closed-connection fixture must observe retry traffic without a fourth request: {}",
        server.fallback_stream_attempts()
    );
}

#[tokio::test]
async fn pause_during_single_stream_retry_backoff_prevents_another_attempt() {
    let server = DeterministicHttpServer::start_concurrent();
    server.set_fallback_stream_hold(true);
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-probe-only-unavailable", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit task");
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while server.fallback_stream_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "first fallback stream attempt did not arrive"
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let pause_result = engine.pause(task_id);
    server.set_fallback_stream_hold(false);
    pause_result.expect("pause during retry backoff");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("wait for fallback pause");
    tokio::time::sleep(Duration::from_millis(250)).await;

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert_eq!(
        server.fallback_stream_attempts(),
        1,
        "pause must win the retry backoff without opening another stream"
    );
}

#[tokio::test]
async fn cancel_during_single_stream_retry_backoff_prevents_another_attempt() {
    let server = DeterministicHttpServer::start_concurrent();
    server.set_fallback_stream_failures(u64::MAX);
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-probe-only-unavailable", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit task");
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while server.fallback_stream_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "first fallback stream attempt did not arrive"
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    engine.cancel(task_id).expect("cancel during retry backoff");
    let cancelled = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("wait for fallback cancellation");
    tokio::time::sleep(Duration::from_millis(250)).await;

    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert_eq!(
        server.fallback_stream_attempts(),
        1,
        "cancel must win the retry backoff without opening another stream"
    );
    assert!(
        std::fs::read_dir(&destination)
            .expect("read cancelled destination")
            .next()
            .is_none(),
        "cancel must remove fallback temporary work"
    );
}

#[tokio::test]
async fn range_ignored_after_another_segment_progress_restarts_cleanly_as_a_single_stream() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-progress-then-ignored", server.address());
    request.connection_limit = Some(4);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit mixed-replica task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for clean fallback");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(
        completed.downloaded_bytes,
        concurrent_fixture_bytes().len() as u64
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read fallback artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn changed_resource_after_a_checkpoint_fails_without_publishing_or_discarding_progress() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-mutable?checkpoint=1", server.address());
    request.speed_limit_bytes_per_second = Some(64 * 1024);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit mutable task");
    wait_for_partial_progress(&engine, task_id, concurrent_fixture_bytes().len() as u64).await;
    engine.pause(task_id).expect("pause mutable task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused task");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    let checkpoint = paused.downloaded_bytes;
    let temporary_path = paused
        .temporary_path
        .clone()
        .expect("temporary artifact is tracked");
    assert!(checkpoint > 0 && checkpoint < concurrent_fixture_bytes().len() as u64);
    server.advance_resource_revision();
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    reopened.resume(task_id).expect("resume mutable task");
    let failed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for rejected resume");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert_eq!(failed.downloaded_bytes, checkpoint);
    assert!(temporary_path.exists());
    assert!(!destination.join("engine-fixture.bin").exists());
}

#[tokio::test]
async fn crash_window_bytes_ahead_of_sqlite_checkpoint_are_safely_overwritten_on_resume() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create crash recovery directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut ranged_request = request(&server, destination.clone());
    ranged_request.source_url = format!(
        "http://{}/range-resume-required?checkpoint=1",
        server.address()
    );
    ranged_request.speed_limit_bytes_per_second = Some(64 * 1024);

    let task_id = engine
        .submit(ranged_request, RequestOrigin::Ui)
        .expect("submit crash-window task");
    wait_for_partial_progress(&engine, task_id, concurrent_fixture_bytes().len() as u64).await;
    engine.pause(task_id).expect("pause at durable checkpoint");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for durable pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    let checkpoint = paused.downloaded_bytes;
    let temporary_path = paused
        .temporary_path
        .clone()
        .expect("temporary artifact is tracked");
    assert!(checkpoint > 0 && checkpoint < concurrent_fixture_bytes().len() as u64);
    drop(engine);

    let mut artifact = std::fs::OpenOptions::new()
        .write(true)
        .open(&temporary_path)
        .expect("open crash-window artifact");
    artifact
        .seek(std::io::SeekFrom::Start(checkpoint))
        .expect("seek past durable checkpoint");
    artifact
        .write_all(&vec![
            0xff;
            concurrent_fixture_bytes().len() - checkpoint as usize
        ])
        .expect("write uncheckpointed crash-window bytes");
    artifact.sync_all().expect("sync simulated crash bytes");
    drop(artifact);

    let connection = rusqlite::Connection::open(&database_path).expect("open crash database");
    connection
        .execute(
            "UPDATE download_tasks SET status = 'downloading' WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )
        .expect("restore interrupted task state");
    connection
        .execute(
            "UPDATE download_segments SET state = 'active' WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )
        .expect("restore interrupted segment state");
    drop(connection);

    let reopened = DownloadEngine::open(&database_path).expect("recover crashed engine");
    let recovered = reopened
        .task(task_id)
        .expect("read recovered task")
        .expect("recovered task exists");
    assert_eq!(recovered.task.status(), DownloadStatus::Paused);
    assert_eq!(recovered.downloaded_bytes, checkpoint);

    reopened
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before recovery");
    reopened.resume(task_id).expect("resume recovered task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for crash recovery");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "crash recovery failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read recovered artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn forcibly_terminated_download_process_recovers_and_resumes_from_sqlite() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create forced-exit directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let status = Command::new(std::env::current_exe().expect("locate contract test process"))
        .args([
            "--exact",
            "crash_helper_process",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NXTOOLS_DOWNLOAD_CRASH_HELPER", "1")
        .env("NXTOOLS_DOWNLOAD_CRASH_DB", &database_path)
        .env("NXTOOLS_DOWNLOAD_CRASH_DESTINATION", &destination)
        .env(
            "NXTOOLS_DOWNLOAD_CRASH_URL",
            format!("http://{}/range-disconnect", server.address()),
        )
        .status()
        .expect("run crash-helper process");
    assert_eq!(
        status.code(),
        Some(86),
        "helper must terminate without running Rust destructors"
    );

    let engine = DownloadEngine::open(&database_path).expect("recover force-terminated engine");
    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: Default::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
        panic!("recovered engine must return a snapshot");
    };
    let recovered = snapshot
        .task_details
        .into_iter()
        .find(|task| task.task.request().file_name.as_deref() == Some("forced-exit.bin"))
        .expect("recovered forced-exit task");
    assert_eq!(recovered.task.status(), DownloadStatus::Paused);
    assert_eq!(recovered.downloaded_bytes, 0);
    let temporary_path = recovered
        .temporary_path
        .as_ref()
        .expect("forced-exit artifact is tracked");
    assert!(temporary_path.exists());
    assert!(!destination.join("forced-exit.bin").exists());

    engine
        .resume(recovered.task.id())
        .expect("resume force-terminated task");
    let completed = engine
        .wait_for_terminal(recovered.task.id(), Duration::from_secs(5))
        .await
        .expect("wait for forced-exit recovery");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "forced-exit recovery failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("forced-exit.bin")).expect("read force-terminated recovery"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn task_can_resume_after_network_disconnect_exhausts_the_current_attempt() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create disconnect recovery directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut disconnected_request = request(&server, destination.clone());
    disconnected_request.source_url = format!("http://{}/range-disconnect", server.address());
    disconnected_request.file_name = Some("disconnect.bin".into());

    let task_id = engine
        .submit(disconnected_request, RequestOrigin::Ui)
        .expect("submit disconnect task");
    wait_for_progress(&engine, task_id).await;
    server.set_network_available(false);
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(30))
        .await
        .expect("wait for disconnected task failure");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert!(
        failed.downloaded_bytes > 0
            && failed.downloaded_bytes < concurrent_fixture_bytes().len() as u64
    );
    let retained = failed
        .temporary_path
        .as_ref()
        .expect("retained interrupted artifact");
    let prefix = std::fs::read(retained).expect("read retained prefix");
    let written = failed.downloaded_bytes as usize;
    assert_eq!(&prefix[..written], &concurrent_fixture_bytes()[..written]);
    assert!(
        failed.error.as_ref().is_some_and(|fault| fault.retryable),
        "network disconnect must remain retryable: {:?}",
        failed.error
    );
    assert!(!destination.join("disconnect.bin").exists());

    server.set_network_available(true);
    engine.resume(task_id).expect("resume after reconnect");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for reconnected task");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "reconnect recovery failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("disconnect.bin")).expect("read reconnected artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn connection_limit_runs_multiple_ranges_concurrently_into_one_verified_file() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit concurrent task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for concurrent task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "concurrent transfer failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read concurrent artifact"),
        concurrent_fixture_bytes()
    );
    assert!(
        server.max_concurrent_range_transfers() >= 2,
        "connection limit above one must produce overlapping Range transfers"
    );
}

#[tokio::test]
async fn fresh_multi_connection_download_avoids_if_range_incompatible_cdn_fallback() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!(
        "http://{}/range-concurrent-if-range-incompatible",
        server.address()
    );
    request.connection_limit = Some(4);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit conditional-incompatible task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for conditional-incompatible task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "fresh multi-connection transfer failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read conditional-incompatible artifact"),
        concurrent_fixture_bytes()
    );
    assert!(
        server.max_concurrent_range_transfers() >= 2,
        "fresh transfer must keep overlapping ranges instead of falling back to one stream"
    );
}

#[tokio::test]
async fn persisted_resume_restarts_safely_when_if_range_is_incompatible_with_cdn() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!(
        "http://{}/range-concurrent-if-range-incompatible",
        server.address()
    );
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(64 * 1024);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit conditional-incompatible task");
    wait_for_partial_progress(&engine, task_id, concurrent_fixture_bytes().len() as u64).await;
    engine
        .pause(task_id)
        .expect("pause conditional-incompatible task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for conditional-incompatible pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0);
    assert!(paused.downloaded_bytes < concurrent_fixture_bytes().len() as u64);
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    reopened
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before resume");
    reopened
        .resume(task_id)
        .expect("resume conditional-incompatible task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(6))
        .await
        .expect("wait for conditional-incompatible resume");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "conditional-incompatible resume failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read conditional-incompatible resumed artifact"),
        concurrent_fixture_bytes()
    );
    assert!(
        server.max_concurrent_range_transfers() >= 2,
        "safe restart should return to fresh overlapping Range requests"
    );
}

#[tokio::test]
async fn one_task_speed_limit_is_shared_by_all_of_its_range_workers() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(256 * 1024);
    let started = std::time::Instant::now();

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit rate-limited task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for rate-limited task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "rate-limited transfer failed: {:?}",
        completed.error
    );
    assert!(
        started.elapsed() >= Duration::from_millis(750),
        "four workers must share the task limit instead of each receiving the full budget"
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read rate-limited artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn one_global_speed_limit_is_shared_by_all_active_tasks() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    engine
        .set_global_speed_limit(Some(512 * 1024))
        .expect("set global speed limit");
    let started = std::time::Instant::now();

    let mut first = request(&server, directory.path().join("first"));
    first.source_url = format!("http://{}/range-concurrent", server.address());
    first.connection_limit = Some(4);
    let mut second = request(&server, directory.path().join("second"));
    second.source_url = format!("http://{}/range-concurrent", server.address());
    second.connection_limit = Some(4);
    let first_id = engine
        .submit(first, RequestOrigin::Ui)
        .expect("submit first globally limited task");
    let second_id = engine
        .submit(second, RequestOrigin::Ui)
        .expect("submit second globally limited task");

    let (first_task, second_task) = tokio::join!(
        engine.wait_for_terminal(first_id, Duration::from_secs(4)),
        engine.wait_for_terminal(second_id, Duration::from_secs(4))
    );
    assert_eq!(
        first_task.expect("wait for first task").task.status(),
        DownloadStatus::Completed
    );
    assert_eq!(
        second_task.expect("wait for second task").task.status(),
        DownloadStatus::Completed
    );
    assert!(
        started.elapsed() >= Duration::from_millis(750),
        "active tasks must share the global budget"
    );
}

#[tokio::test]
async fn changing_a_running_task_speed_limit_takes_effect_without_restarting_it() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(32 * 1024);
    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit slowly limited task");
    wait_for_progress(&engine, task_id).await;
    let changed_at = std::time::Instant::now();

    engine
        .set_task_speed_limit(task_id, None)
        .expect("remove running task limit");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait after live task limit update");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert!(
        changed_at.elapsed() < Duration::from_secs(2),
        "removing a task limit must wake current workers without a restart"
    );
    assert_eq!(completed.task.request().speed_limit_bytes_per_second, None);
}

#[tokio::test]
async fn a_single_stream_download_uses_the_same_task_speed_limit() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/no-range", server.address());
    request.speed_limit_bytes_per_second = Some(16);
    let started = std::time::Instant::now();

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit limited single stream");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for limited single stream");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert!(
        started.elapsed() >= Duration::from_millis(800),
        "single-stream downloads must pass through the task limiter"
    );
}

#[tokio::test]
async fn pause_interrupts_a_worker_waiting_for_bandwidth() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(8 * 1024);
    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit bandwidth-waiting task");
    wait_for_progress(&engine, task_id).await;
    let paused_at = std::time::Instant::now();

    engine.pause(task_id).expect("pause bandwidth-waiting task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("bandwidth wait must observe pause");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        paused_at.elapsed() < Duration::from_millis(500),
        "pause must not wait for the next bandwidth permit"
    );
}

#[tokio::test]
async fn global_and_task_speed_limits_survive_engine_restart() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    engine
        .set_global_speed_limit(Some(768 * 1024))
        .expect("persist global limit");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/archive.bin".to_owned(),
                save_directory: directory.path().join("artifacts"),
                save_directory_explicit: false,
                file_name: Some("archive.bin".to_owned()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(4),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Ui,
        )
        .expect("create queued task");
    engine
        .set_task_speed_limit(task_id, Some(128 * 1024))
        .expect("persist task limit");
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    assert_eq!(reopened.global_speed_limit(), Some(768 * 1024));
    assert_eq!(
        reopened
            .task(task_id)
            .expect("read restored task")
            .expect("restored task exists")
            .task
            .request()
            .speed_limit_bytes_per_second,
        Some(128 * 1024)
    );
}

#[tokio::test]
async fn dynamic_rebalance_splits_a_slow_tail_without_overlapping_output() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-imbalanced", server.address());
    request.connection_limit = Some(2);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit imbalanced task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(6))
        .await
        .expect("wait for dynamically balanced task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "dynamic transfer failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read dynamically balanced artifact"),
        imbalanced_fixture_bytes()
    );
    assert!(
        server.dynamic_range_attempts() >= 4,
        "a released connection slot must split the slow unwritten tail; attempts={}",
        server.dynamic_range_attempts()
    );
}

#[tokio::test]
async fn stored_adaptive_policy_does_not_disable_http_claim_bounded_tail_splits() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");

    for (label, policy) in [
        (
            "disabled",
            DownloadAdaptiveSegmentPolicy {
                enabled: false,
                max_segments: 32,
                min_segment_mib: 3,
            },
        ),
        (
            "at-ceiling",
            DownloadAdaptiveSegmentPolicy {
                enabled: true,
                max_segments: 2,
                min_segment_mib: 3,
            },
        ),
        (
            "tail-too-small",
            DownloadAdaptiveSegmentPolicy {
                enabled: true,
                max_segments: 32,
                min_segment_mib: 5,
            },
        ),
    ] {
        let attempts_before = server.dynamic_range_attempts();
        let destination = directory.path().join(label);
        let mut request = request(&server, destination.clone());
        request.source_url = format!("http://{}/range-imbalanced", server.address());
        request.file_name = Some(format!("{label}.bin"));
        request.connection_limit = Some(2);
        request.adaptive_segment_policy = Some(policy);

        let task_id = engine
            .submit(request, RequestOrigin::Ui)
            .expect("submit policy fixture");
        let completed = engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("wait for policy fixture");

        assert_eq!(
            completed.task.status(),
            DownloadStatus::Completed,
            "{label}"
        );
        assert!(
            server.dynamic_range_attempts() - attempts_before >= 4,
            "{label} remains serialized policy, not an HTTP planner split gate"
        );
        assert_eq!(
            std::fs::read(destination.join(format!("{label}.bin"))).expect("read policy fixture"),
            imbalanced_fixture_bytes()
        );
    }
}

#[tokio::test]
async fn a_dynamically_split_plan_resumes_after_engine_restart() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-imbalanced", server.address());
    request.connection_limit = Some(2);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit imbalanced task");
    let split_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while server.dynamic_range_attempts() < 4 {
        assert!(
            tokio::time::Instant::now() < split_deadline,
            "dynamic split did not start before pause"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause dynamically split task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for dynamic pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0);
    assert!(paused.downloaded_bytes < imbalanced_fixture_bytes().len() as u64);
    let detail = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::TaskDetail { task_id },
    });
    let DownloadCommandOutcome::TaskDetail(detail) = detail.outcome else {
        panic!("paused task detail must remain available");
    };
    assert!(
        detail
            .segments
            .iter()
            .all(|segment| segment.state != DownloadSegmentState::Active),
        "paused queued and running segments must all persist as non-active"
    );
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    reopened
        .resume(task_id)
        .expect("resume dynamically split task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(6))
        .await
        .expect("wait for dynamically split resume");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "dynamic resume failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read resumed dynamic artifact"),
        imbalanced_fixture_bytes()
    );
}

#[tokio::test]
async fn concurrent_ranges_resume_all_persisted_segments_after_engine_restart() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(64 * 1024);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit concurrent task");
    wait_for_partial_progress(&engine, task_id, concurrent_fixture_bytes().len() as u64).await;
    engine.pause(task_id).expect("pause concurrent task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for concurrent pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes >= 4);
    assert!(paused.downloaded_bytes < concurrent_fixture_bytes().len() as u64);
    let checkpoint = paused.downloaded_bytes;
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("read task")
        .expect("persisted task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Paused);
    assert_eq!(restored.downloaded_bytes, checkpoint);
    reopened
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before resume");

    reopened.resume(task_id).expect("resume concurrent task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for concurrent resume");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "concurrent resume failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read resumed artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn transient_range_failures_retry_without_losing_the_task_or_artifact() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-transient", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit transient task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for retried task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "transient transfer failed after {} attempts: {:?}",
        server.transient_range_attempts(),
        completed.error,
    );
    assert_eq!(server.transient_range_attempts(), 2);
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read retried artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn equivalent_cdn_replica_validator_does_not_restart_a_fresh_segment() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-validator-replica", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit validator-replica task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for validator-replica retry");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "validator mismatch failed instead of retrying: {:?}",
        completed.error
    );
    assert_eq!(server.validator_range_attempts(), 1);
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read validator-retried artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn fresh_concurrent_download_accepts_valid_ranges_from_equivalent_cdn_replicas() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!(
        "http://{}/range-concurrent-varying-validator",
        server.address()
    );
    request.connection_limit = Some(8);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit varying-replica task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for varying-replica task");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(
        server.varying_validator_full_stream_attempts(),
        0,
        "valid concurrent ranges must not be discarded and downloaded again as a full stream"
    );
    assert!(
        server.max_concurrent_range_transfers() >= 2,
        "the transfer must retain concurrent range requests"
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read completed artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn pausing_during_retry_backoff_stops_new_network_attempts() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-always-transient", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit retrying task");
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while server.transient_range_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "first transient attempt did not arrive"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause during retry backoff");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("wait for retry pause");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        server.transient_range_attempts() < 3,
        "pause must interrupt backoff before the retry budget is exhausted"
    );
}

#[tokio::test]
async fn permanent_range_failure_is_not_retried() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-permanent", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit permanent failure task");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("wait for permanent failure");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert_eq!(
        server.transient_range_attempts(),
        1,
        "permanent HTTP failures must not consume the retry budget"
    );
}

#[tokio::test]
async fn cancelled_task_cleans_temporary_work_and_can_be_removed_from_history() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut slow_request = request(&server, destination.clone());
    slow_request.source_url = format!("http://{}/stall", server.address());

    let task_id = engine
        .submit(slow_request, RequestOrigin::Ui)
        .expect("submit slow task");
    wait_for_progress(&engine, task_id).await;
    engine.cancel(task_id).expect("cancel task");
    let cancelled = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for cancelled task");

    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert!(!destination.join("engine-fixture.bin").exists());
    assert_eq!(
        std::fs::read_dir(&destination)
            .expect("read destination")
            .count(),
        0
    );

    engine
        .remove(task_id, false)
        .expect("remove cancelled task");
    assert!(engine.task(task_id).expect("read task").is_none());
}

#[tokio::test]
async fn opening_the_engine_recovers_and_resumes_an_orphan_interrupted_task() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let store = SqliteDownloadStore::open(&database_path).expect("open task store");
    let mut task = DownloadTask::new(
        request(&server, directory.path().join("artifacts")),
        RequestOrigin::Ui,
        1_000,
    );
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
    ] {
        task.transition_to(status, 1_100)
            .expect("build interrupted task");
    }
    store.save_task(&task).expect("persist interrupted task");
    drop(store);

    let engine = DownloadEngine::open(&database_path).expect("open recovered engine");
    let recovered = engine
        .task(task.id())
        .expect("read task")
        .expect("recovered task exists");

    assert_eq!(recovered.task.status(), DownloadStatus::Paused);
    assert!(!recovered.queue_eligible);
    engine
        .resume(task.id())
        .expect("resume reconciled recovered task");
    let completed = engine
        .wait_for_terminal(task.id(), Duration::from_secs(2))
        .await
        .expect("wait for reconciled task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
}

#[tokio::test]
async fn server_derived_artifact_paths_survive_restart_and_cancel_cleans_the_exact_temp_file() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut slow_request = request(&server, destination.clone());
    slow_request.source_url = format!("http://{}/stall", server.address());
    slow_request.file_name = None;

    let task_id = engine
        .submit(slow_request, RequestOrigin::Ui)
        .expect("submit slow task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("pause task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for pause");
    let temporary_path = paused
        .temporary_path
        .clone()
        .expect("temporary artifact path is tracked");
    assert!(temporary_path.exists());
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("read restored task")
        .expect("restored task exists");
    assert_eq!(restored.final_path, Some(destination.join("stall")));
    assert_eq!(
        restored.temporary_path.as_deref(),
        Some(temporary_path.as_path())
    );
    reopened.cancel(task_id).expect("cancel restored task");
    assert!(!temporary_path.exists());
}

#[tokio::test]
async fn reject_strategy_preserves_an_existing_destination_without_overwriting_it() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    std::fs::create_dir_all(&destination).expect("create destination");
    let final_path = destination.join("engine-fixture.bin");
    std::fs::write(&final_path, b"keep-me").expect("seed destination");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");

    let mut duplicate_request = request(&server, destination);
    duplicate_request.duplicate_strategy = DownloadDuplicateStrategy::Reject;
    let error = engine
        .submit(duplicate_request, RequestOrigin::Ui)
        .expect_err("occupied target must be rejected before task creation");
    assert!(matches!(
        error,
        nxtools_download::DownloadEngineError::DownloadOrganizationPolicy(
            nxtools_download::DownloadOrganizationPolicyError::TargetOccupied
        )
    ));
    assert!(engine.tasks().expect("read tasks").is_empty());
    assert_eq!(
        std::fs::read(final_path).expect("read destination"),
        b"keep-me"
    );
}

#[test]
fn engine_session_generation_changes_after_engine_restart() {
    let directory = tempfile::tempdir().expect("create generation database directory");
    let database = directory.path().join("downloads.sqlite3");
    let first = DownloadEngine::open(&database).expect("open first engine session");
    let first_snapshot = first.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(first_snapshot) = first_snapshot.outcome else {
        panic!("first engine must return a snapshot");
    };
    let first_generation = first_snapshot.generation;
    drop(first);

    let second = DownloadEngine::open(&database).expect("open second engine session");
    let second_snapshot = second.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(second_snapshot) = second_snapshot.outcome else {
        panic!("second engine must return a snapshot");
    };
    assert_ne!(first_generation, second_snapshot.generation);
    assert_ne!(second_snapshot.generation, 0);
}

#[test]
fn subscribe_with_a_stale_generation_requires_a_snapshot() {
    let engine = DownloadEngine::open_in_memory().expect("open generation engine");
    let events = engine
        .take_event_receiver()
        .expect("attach generation event receiver");
    let generation = engine.session_generation();
    let stale_generation = generation.wrapping_add(1).max(1);
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Subscribe {
            after_sequence: Some(0),
            generation: Some(stale_generation),
        },
    });
    assert!(matches!(
        result.outcome,
        DownloadCommandOutcome::Subscribed {
            generation: observed,
            snapshot_required: true,
            ..
        } if observed == generation
    ));
    let event = events
        .recv_blocking()
        .expect("receive stale-generation snapshot event");
    assert_eq!(event.generation, generation);
    assert!(matches!(
        event.event,
        DownloadEvent::SnapshotRequired {
            expected_generation: Some(expected),
            observed_generation: Some(observed),
            ..
        } if expected == generation && observed == stale_generation
    ));
}

#[test]
fn subscribe_with_same_generation_and_sequence_gap_requires_a_snapshot() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create sequence-gap directory");
    let engine = DownloadEngine::open_in_memory().expect("open sequence-gap engine");
    let events = engine
        .take_event_receiver()
        .expect("attach sequence-gap receiver");
    let generation = engine.session_generation();
    let accepted = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Submit {
            request: request(&server, directory.path().join("artifacts")),
            mode: DownloadSubmitMode::QueueOnly,
            idempotency_key: None,
        },
    });
    assert!(matches!(
        accepted.outcome,
        DownloadCommandOutcome::Accepted { task_id: Some(_) }
    ));
    let _ = events.recv_blocking().expect("receive accepted task event");

    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Subscribe {
            after_sequence: Some(0),
            generation: Some(generation),
        },
    });
    assert!(matches!(
        result.outcome,
        DownloadCommandOutcome::Subscribed {
            snapshot_required: true,
            generation: observed,
            ..
        } if observed == generation
    ));
    let event = events
        .recv_blocking()
        .expect("receive sequence-gap snapshot event");
    assert!(matches!(
        event.event,
        DownloadEvent::SnapshotRequired {
            expected_sequence: 1,
            observed_sequence: 1,
            expected_generation: Some(expected),
            observed_generation: Some(observed),
        } if expected == generation && observed == generation
    ));
}

#[tokio::test]
async fn versioned_commands_publish_sequenced_state_events() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let events = engine
        .take_event_receiver()
        .expect("attach event subscriber");
    assert!(
        engine.take_event_receiver().is_err(),
        "the in-process event stream has one owning subscriber"
    );
    let request_id = uuid::Uuid::new_v4();

    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id,
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Submit {
            request: request(&server, directory.path().join("artifacts")),
            mode: DownloadSubmitMode::StartNow,
            idempotency_key: None,
        },
    });
    let task_id = match result.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } => task_id,
        outcome => panic!("unexpected command outcome: {outcome:?}"),
    };
    let first = events.recv().await.expect("receive task event");
    assert_eq!(first.sequence, 1);
    assert!(matches!(
        first.event,
        DownloadEvent::TaskChanged { task } if task.id() == task_id
    ));

    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Snapshot {
            query: Default::default(),
        },
    });
    assert!(matches!(
        snapshot.outcome,
        DownloadCommandOutcome::Snapshot(snapshot)
            if snapshot.task_details.iter().any(|task| task.task.id() == task_id)
    ));

    let pause = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Task {
            task_id,
            action: DownloadTaskAction::Cancel,
        },
    });
    assert!(matches!(
        pause.outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
}

#[tokio::test]
async fn queue_only_submission_stays_created_until_resumed() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");

    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Submit {
            request: request(&server, destination.clone()),
            mode: DownloadSubmitMode::QueueOnly,
            idempotency_key: None,
        },
    });
    let task_id = match result.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } => task_id,
        outcome => panic!("unexpected command outcome: {outcome:?}"),
    };

    tokio::time::sleep(Duration::from_millis(40)).await;
    let queued = engine
        .task(task_id)
        .expect("read queued task")
        .expect("queued task exists");
    assert_eq!(queued.task.status(), DownloadStatus::Created);
    assert!(!destination.join("engine-fixture.bin").exists());

    engine.resume(task_id).expect("start queued task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for queued task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
}

#[tokio::test]
async fn queue_only_submission_can_be_cancelled_without_starting_network_work() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let task_id = engine
        .enqueue(request(&server, destination.clone()), RequestOrigin::Ui)
        .expect("queue task");

    engine.cancel(task_id).expect("cancel queued task");
    let cancelled = engine
        .task(task_id)
        .expect("read task")
        .expect("task exists");
    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert!(!destination.join("engine-fixture.bin").exists());
}

async fn wait_for_progress(engine: &DownloadEngine, task_id: nxtools_download::DownloadTaskId) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let task = engine
            .task(task_id)
            .expect("read task")
            .expect("task exists");
        if task.downloaded_bytes > 0 {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "task did not publish progress; status={:?}, error={:?}",
            task.task.status(),
            task.error
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
async fn wait_for_partial_progress(
    engine: &DownloadEngine,
    task_id: nxtools_download::DownloadTaskId,
    total_bytes: u64,
) {
    let deadline = std::time::Instant::now() + Duration::from_secs(6);
    loop {
        let snapshot = engine
            .task(task_id)
            .expect("read checkpoint task")
            .expect("checkpoint task exists");
        assert!(
            !matches!(
                snapshot.task.status(),
                DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
            ),
            "task ended before a partial checkpoint; status={:?}, error={:?}",
            snapshot.task.status(),
            snapshot.error
        );
        if snapshot.downloaded_bytes > 0 {
            assert!(snapshot.downloaded_bytes < total_bytes);
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "task did not publish partial progress; status={:?}, error={:?}",
            snapshot.task.status(),
            snapshot.error
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[test]
fn compact_snapshot_rebuilds_list_and_keeps_detail_selected_task_only() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create compact snapshot directory");
    let engine = DownloadEngine::open_in_memory().expect("open compact snapshot engine");
    let first = engine
        .enqueue(
            request(&server, directory.path().join("first")),
            RequestOrigin::Internal,
        )
        .expect("enqueue first compact task");
    let second = engine
        .enqueue(
            request(&server, directory.path().join("second")),
            RequestOrigin::Internal,
        )
        .expect("enqueue second compact task");

    let compact = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    });
    let DownloadCommandOutcome::Snapshot(compact) = compact.outcome else {
        panic!("compact snapshot expected");
    };
    assert!(compact.tasks.is_empty());
    assert!(compact.task_details.is_empty());
    assert_eq!(
        compact
            .task_summaries
            .iter()
            .map(|summary| summary.task_id)
            .collect::<Vec<_>>(),
        vec![first, second]
    );

    let detail = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::TaskDetail { task_id: second },
    });
    let DownloadCommandOutcome::TaskDetail(detail) = detail.outcome else {
        panic!("selected task detail expected");
    };
    assert_eq!(detail.task.task.id(), second);

    let legacy = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(legacy) = legacy.outcome else {
        panic!("legacy snapshot expected");
    };
    assert_eq!(legacy.task_details.len(), 2);
    assert!(legacy.task_summaries.is_empty());
}
#[tokio::test]
async fn compact_snapshot_preserves_final_path_and_searches_that_path() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create final-path snapshot directory");
    let engine = DownloadEngine::open_in_memory().expect("open final-path snapshot engine");
    let mut download = request(&server, directory.path().to_owned());
    download.file_name = Some("compact-final.bin".into());
    let task_id = engine
        .submit(download, RequestOrigin::Internal)
        .expect("submit final-path snapshot task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for final-path snapshot task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    let final_path = directory.path().join("compact-final.bin");

    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
        panic!("compact final-path snapshot expected");
    };
    let summary = snapshot
        .task_summaries
        .iter()
        .find(|summary| summary.task_id == task_id)
        .expect("final-path summary");
    assert_eq!(summary.final_path.as_ref(), Some(&final_path));
    assert_eq!(
        summary
            .clone()
            .into_engine_task()
            .expect("project final-path summary")
            .final_path
            .as_ref(),
        Some(&final_path)
    );

    let search = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery {
                search: Some(directory.path().to_string_lossy().into_owned()),
                ..DownloadQuery::compact()
            },
        },
    });
    let DownloadCommandOutcome::Snapshot(search) = search.outcome else {
        panic!("compact final-path search snapshot expected");
    };
    assert_eq!(
        search
            .task_summaries
            .iter()
            .map(|summary| summary.task_id)
            .collect::<Vec<_>>(),
        vec![task_id]
    );
}

#[test]
fn compact_snapshot_is_self_contained_after_a_sequence_gap_refresh() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create compact gap directory");
    let engine = DownloadEngine::open_in_memory().expect("open compact gap engine");
    let first = engine
        .enqueue(
            request(&server, directory.path().join("first")),
            RequestOrigin::Internal,
        )
        .expect("enqueue first compact gap task");
    let first_snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    });
    let DownloadCommandOutcome::Snapshot(first_snapshot) = first_snapshot.outcome else {
        panic!("compact gap baseline snapshot expected");
    };
    assert_eq!(first_snapshot.task_summaries.len(), 1);

    let second = engine
        .enqueue(
            request(&server, directory.path().join("second")),
            RequestOrigin::Internal,
        )
        .expect("enqueue second compact gap task");
    let subscription = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Subscribe {
            after_sequence: Some(first_snapshot.sequence),
            generation: Some(first_snapshot.generation),
        },
    });
    assert!(matches!(
        subscription.outcome,
        DownloadCommandOutcome::Subscribed {
            snapshot_required: true,
            ..
        }
    ));

    let refreshed = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    });
    let DownloadCommandOutcome::Snapshot(refreshed) = refreshed.outcome else {
        panic!("compact gap refresh snapshot expected");
    };
    assert_eq!(
        refreshed
            .task_summaries
            .iter()
            .map(|summary| summary.task_id)
            .collect::<Vec<_>>(),
        vec![first, second]
    );
    assert!(refreshed.task_details.is_empty());
}

fn snapshot_task_ids(
    engine: &DownloadEngine,
    query: DownloadQuery,
) -> Vec<nxtools_download::DownloadTaskId> {
    let compact = query.compact;
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot { query },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = result.outcome else {
        panic!("snapshot query must return a snapshot");
    };
    if compact {
        snapshot
            .task_summaries
            .into_iter()
            .map(|summary| summary.task_id)
            .collect()
    } else {
        snapshot
            .task_details
            .into_iter()
            .map(|task| task.task.id())
            .collect()
    }
}

#[test]
fn compact_snapshot_preserves_legacy_filtering_and_queue_order() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create snapshot equivalence directory");
    let engine = DownloadEngine::open_in_memory().expect("open snapshot equivalence engine");
    engine
        .update_download_organization_policy(&DownloadSettings {
            default_directory: directory.path().to_string_lossy().into_owned(),
            categories: vec![
                DownloadCategorySettings {
                    id: "release_archive".into(),
                    name: "release-archive".into(),
                    enabled: true,
                    save_directory: "release".into(),
                    extensions: Vec::new(),
                    url_patterns: Vec::new(),
                },
                DownloadCategorySettings {
                    id: "mirror_cache".into(),
                    name: "mirror-cache".into(),
                    enabled: true,
                    save_directory: "mirror".into(),
                    extensions: Vec::new(),
                    url_patterns: Vec::new(),
                },
                DownloadCategorySettings {
                    id: "archive_category".into(),
                    name: "archive-category".into(),
                    enabled: true,
                    save_directory: "archive".into(),
                    extensions: Vec::new(),
                    url_patterns: Vec::new(),
                },
            ],
            ..DownloadSettings::default()
        })
        .expect("register snapshot equivalence categories");
    let make_request = |source_url: &str, file_name: &str, category_name: &str| {
        let mut request = request(&server, directory.path().join(file_name));
        request.source_url = source_url.to_owned();
        request.file_name = Some(file_name.to_owned());
        request.category_id = Some(category_name.replace('-', "_"));
        request.category_name = Some(category_name.to_owned());
        request
    };
    let first = engine
        .enqueue(
            make_request(
                "https://release.example.test/files/zeta.bin",
                "zeta.bin",
                "release-archive",
            ),
            RequestOrigin::Internal,
        )
        .expect("enqueue first equivalence task");
    let second = engine
        .enqueue(
            make_request(
                "https://mirror.example.test/files/alpha.bin",
                "alpha.bin",
                "mirror-cache",
            ),
            RequestOrigin::Internal,
        )
        .expect("enqueue second equivalence task");
    let third = engine
        .enqueue(
            make_request(
                "https://archive.example.test/files/other.bin",
                "other.bin",
                "archive-category",
            ),
            RequestOrigin::Internal,
        )
        .expect("enqueue third equivalence task");

    let legacy_all = snapshot_task_ids(&engine, DownloadQuery::default());
    let compact_all = snapshot_task_ids(&engine, DownloadQuery::compact());
    assert_eq!(legacy_all, vec![first, second, third]);
    assert_eq!(compact_all, legacy_all);

    for search in ["release.example", "alpha.bin", "archive-category"] {
        let legacy = snapshot_task_ids(
            &engine,
            DownloadQuery {
                search: Some(search.to_owned()),
                ..DownloadQuery::default()
            },
        );
        let compact = snapshot_task_ids(
            &engine,
            DownloadQuery {
                search: Some(search.to_owned()),
                ..DownloadQuery::compact()
            },
        );
        assert_eq!(compact, legacy, "search result differs for {search}");
    }

    engine
        .cancel(third)
        .expect("cancel status equivalence task");
    let legacy_cancelled = snapshot_task_ids(
        &engine,
        DownloadQuery {
            statuses: vec![DownloadStatus::Cancelled],
            ..DownloadQuery::default()
        },
    );
    let compact_cancelled = snapshot_task_ids(
        &engine,
        DownloadQuery {
            statuses: vec![DownloadStatus::Cancelled],
            ..DownloadQuery::compact()
        },
    );
    assert_eq!(compact_cancelled, vec![third]);
    assert_eq!(compact_cancelled, legacy_cancelled);

    let legacy_limited = snapshot_task_ids(
        &engine,
        DownloadQuery {
            limit: Some(2),
            ..DownloadQuery::default()
        },
    );
    let compact_limited = snapshot_task_ids(
        &engine,
        DownloadQuery {
            limit: Some(2),
            ..DownloadQuery::compact()
        },
    );
    assert_eq!(compact_limited, vec![first, second]);
    assert_eq!(compact_limited, legacy_limited);

    let detail = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::TaskDetail { task_id: first },
    });
    let DownloadCommandOutcome::TaskDetail(detail) = detail.outcome else {
        panic!("selected task detail expected");
    };
    assert_eq!(
        detail.task,
        engine
            .task(first)
            .expect("read selected equivalence task")
            .expect("selected equivalence task exists")
    );
}

#[tokio::test]
async fn ref06_existing_http_hls_dash_waiters_wake_without_restarting_tasks() {
    for protocol in ["http", "hls", "dash"] {
        let server = DeterministicHttpServer::start_concurrent();
        let directory = tempfile::tempdir().expect("isolated rate waiter output");
        let destination = directory.path().join("artifacts");
        let engine = DownloadEngine::open(directory.path().join("downloads.sqlite3"))
            .expect("persistent engine");
        let mut limited = request(&server, destination.clone());
        limited.file_name = Some("waiter.bin".into());
        limited.connection_limit = Some(1);
        limited.speed_limit_bytes_per_second = Some(1);
        let expected = match protocol {
            "http" => {
                limited.source_url = format!("http://{}/range-concurrent", server.address());
                concurrent_fixture_bytes()
            }
            "hls" => {
                limited.source_url = format!("http://{}/hls/master.m3u8", server.address());
                limited.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
                    playlist_url: format!(
                        "http://{}/hls/rate-limited/index.m3u8",
                        server.address()
                    ),
                }));
                vec![b'x'; 64 * 1024]
            }
            _ => {
                limited.source_url = format!("http://{}/dash/static.mpd", server.address());
                limited.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
                    manifest_url: limited.source_url.clone(),
                    video_representation_id: Some("v1080".into()),
                    audio_representation_id: None,
                }));
                b"VINIT|V0|V1".to_vec()
            }
        };
        engine
            .set_global_speed_limit(Some(1))
            .expect("manual global ceiling");
        engine
            .set_runtime_speed_cap(Some(1), uuid::Uuid::new_v4())
            .expect("runtime ceiling");
        let task_id = engine
            .submit(limited, RequestOrigin::Ui)
            .expect("start original transfer");
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            let current = engine.task(task_id).expect("query").expect("original task");
            if current.task.status() == DownloadStatus::Downloading && server.total_requests() >= 2
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{protocol} did not reach its real transfer boundary: {:?}",
                current.error
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            engine
                .wait_for_terminal(task_id, Duration::from_millis(120))
                .await
                .is_err(),
            "{protocol} must remain limited"
        );
        engine
            .set_task_speed_limit(task_id, None)
            .expect("release original task waiter");
        assert!(
            engine
                .wait_for_terminal(task_id, Duration::from_millis(120))
                .await
                .is_err(),
            "{protocol} retains global/runtime ceiling"
        );
        engine
            .set_global_speed_limit(None)
            .expect("remove manual ceiling");
        assert_eq!(engine.effective_global_speed_limit(), Some(1));
        assert!(
            engine
                .wait_for_terminal(task_id, Duration::from_millis(120))
                .await
                .is_err(),
            "{protocol} retains runtime ceiling"
        );
        let released_at = std::time::Instant::now();
        engine
            .set_runtime_speed_cap(None, uuid::Uuid::new_v4())
            .expect("wake original aggregate waiters");
        let completed = engine
            .wait_for_terminal(task_id, Duration::from_secs(3))
            .await
            .expect("live waiters must wake");
        assert_eq!(completed.task.id(), task_id);
        assert_eq!(
            completed.task.status(),
            DownloadStatus::Completed,
            "{protocol}: {:?}",
            completed.error
        );
        assert!(released_at.elapsed() < Duration::from_secs(3));
        assert_eq!(completed.task.request().speed_limit_bytes_per_second, None);
        assert_eq!(
            std::fs::read(destination.join("waiter.bin")).expect("original artifact"),
            expected,
            "{protocol} output"
        );
        let reopened = DownloadEngine::open(directory.path().join("downloads.sqlite3"))
            .expect("reopen accepted intent");
        assert_eq!(reopened.global_speed_limit(), None);
        assert_eq!(
            reopened
                .task(task_id)
                .expect("query")
                .expect("persisted task")
                .task
                .request()
                .speed_limit_bytes_per_second,
            None
        );
    }
}

#[test]
fn finishing_progress_unknown_duration_is_event_only_and_backward_compatible() {
    let task_id = nxtools_download::DownloadTaskId::new();
    let legacy = serde_json::json!({
        "task_id": task_id, "downloaded_bytes": 22, "total_bytes": null,
        "bytes_per_second": 0, "active_connections": 0
    });
    let mut progress: nxtools_download::DownloadProgress =
        serde_json::from_value(legacy.clone()).expect("legacy progress event");
    assert_eq!(progress.finishing, None);
    assert_eq!(
        serde_json::to_value(&progress).expect("unchanged transfer event"),
        legacy
    );
    progress.finishing = Some(nxtools_download::DownloadFinishingProgress {
        stage: nxtools_download::DownloadFinishingStage::Muxing,
        completed: 123_000,
        total: None,
    });
    let wire = serde_json::to_value(&progress).expect("unknown media duration event");
    assert_eq!(wire["downloaded_bytes"], 22);
    assert_eq!(wire["finishing"]["total"], serde_json::Value::Null);
    assert_eq!(wire["finishing"]["stage"], "muxing");
}
