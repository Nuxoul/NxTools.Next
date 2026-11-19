//! Debug-only localhost/owned-HWND integration fixture, not a product runtime.
use super::*;

pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    use std::io::{Read, Write};
    let scratch = PathBuf::from(
        std::env::var_os("PI_SCRATCH_DIR").ok_or_else(|| anyhow::anyhow!("scratch required"))?,
    );
    let command_path = scratch.join("fixed-probe-command.json");
    let reply_path = scratch.join("fixed-probe-reply.txt");
    let _ = std::fs::remove_file(&command_path);
    let _ = std::fs::remove_file(&reply_path);
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(12);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                    let mut bytes = [0_u8; 4096];
                    let _ = stream.read(&mut bytes);
                    std::thread::sleep(Duration::from_millis(500));
                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nfixture!");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(_) => break,
            }
        }
    });
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    let commands = endpoint.open_connection_for_contract_test();
    let notifications = cx.new(|_| NotificationCenter::default());
    let application = cx.new(|cx| {
        DownloadApplication::new(
            DownloadEngine::open_in_memory().expect("synthetic engine"),
            Rc::new(platform),
            notifications,
            DownloadRuntimeContext {
                fallback_default_directory: scratch.clone(),
                config: AppConfig::default(),
                config_path: scratch.join("fixed-preview-unused.json"),
                log_directory: scratch.clone(),
                database_path: scratch.join("fixed-preview-unused.sqlite3"),
                ffmpeg_install_directory: scratch.join("fixed-preview-unused-tools"),
            },
            cx,
        )
    });
    let command_application = application.downgrade();
    cx.spawn(async move |cx| {
        while let Ok(envelope) = commands.recv().await {
            let fixed = matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::PresentFixedWindowOverAnchor(_)
                )
            );
            let result = if fixed {
                let _ = std::fs::write(
                    &command_path,
                    serde_json::to_vec(&envelope).expect("fixed wire"),
                );
                let deadline = Instant::now() + Duration::from_secs(8);
                let success = loop {
                    if let Ok(reply) = std::fs::read_to_string(&reply_path) {
                        break reply.trim() == "success";
                    }
                    if Instant::now() >= deadline {
                        break false;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                };
                PlatformCommandResult {
                    success,
                    error: None,
                    native_error_code: None,
                    data: None,
                }
            } else {
                PlatformCommandResult {
                    success: false,
                    error: Some("isolated fixture rejects non-presentation command".into()),
                    native_error_code: None,
                    data: None,
                }
            };
            let _ = command_application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result,
                    },
                    cx,
                );
                if fixed {
                    let application = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let _ = application.update(cx, |application, cx| {
                            if let Some(target) = application.new_download_window {
                                let _ = target.update(cx, |_, window, _| {
                                    println!(
                                        "ui.preview.probe_completed hwnd={} viewport_height={}",
                                        native_window_handle(window).unwrap_or_default(),
                                        window.viewport_size().height.as_f32()
                                    );
                                });
                            }
                        });
                    });
                }
            });
        }
        drop(endpoint);
    })
    .detach();
    application.update(cx, |application, cx| {
        let request = DownloadRequest {
            source_url: format!("http://{address}/fixture.bin"),
            save_directory: scratch,
            save_directory_explicit: false,
            file_name: None,
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
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        application
            .open_new_download_window_with_request(
                Some(NewDownloadInitialSource::Download(request)),
                None,
                cx,
            )
            .expect("isolated probe window");
        let source = application.new_download_window.expect("compact window");
        let _ = source.update(cx, |_, window, _| {
            println!(
                "ui.preview.probe_source hwnd={} scale={}",
                native_window_handle(window).unwrap_or_default(),
                window.scale_factor()
            )
        });
    });
    Ok(())
}
