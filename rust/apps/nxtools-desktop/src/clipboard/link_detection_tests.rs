use super::*;
use crate::clipboard_worker::ClipboardCaptureWorkerHandle;
use gpui_kit::TestAppContext;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
fn capture(text: Option<&str>) -> ClipboardCapture {
    ClipboardCapture {
        text: text.map(str::to_owned),
        image: None,
        paths: Vec::new(),
        native_formats: Vec::new(),
        source_icon_staging_path: None,
        source: ForegroundContext {
            window_handle: 0,
            focus_window_handle: 0,
            process_id: 1,
            process_name: "fixture.exe".into(),
            window_title: "fixture".into(),
        },
        captured_at_unix_ms: nxtools_ipc::unix_time_ms(),
    }
}

fn fixture(
    cx: &mut TestAppContext,
) -> (
    tempfile::TempDir,
    Entity<ClipboardApplication>,
    ClipboardCaptureWorkerHandle,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("link detection fixture");
    let worker = ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("history.db"),
        directory.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 100,
            retention_days: 30,
        },
        1_000,
    )
    .expect("link detection history worker");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(64);
    endpoint.set_connected_for_contract_test(true);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let config = AppConfig::default();
    let path = directory.path().join("config.json");
    config
        .save_to(&path)
        .expect("link detection fixture config");
    let owner = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                config,
                path,
                notifications,
                cx,
            )
        })
    });
    owner.update(cx, |owner, _| {
        owner.link_panel.test_native_window_handle = Some(9002)
    });
    (directory, owner, worker, endpoint)
}

fn drain_detection(
    cx: &mut TestAppContext,
    owner: &Entity<ClipboardApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) {
    ClipboardApplication::wait_for_link_detection(cx, owner);
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        let data = matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::QueryCursorMonitor)
        )
        .then(|| {
            serde_json::to_string(&nxtools_protocol::CursorMonitorSnapshot {
                display_id: 1,
                work_left: 0,
                work_top: 0,
                work_width: 1920,
                work_height: 1040,
                dpi: 96,
            })
            .expect("monitor wire")
        });
        owner.update(cx, |owner, cx| {
            owner.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: envelope.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        data,
                        native_error_code: None,
                    },
                },
                cx,
            )
        });
    }
}

struct HeldProbe {
    entered: mpsc::Receiver<String>,
    release: Option<mpsc::Sender<()>>,
    timed_out: Arc<AtomicBool>,
}

impl HeldProbe {
    fn release(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }

    fn next(&self) -> String {
        self.entered
            .recv_timeout(Duration::from_secs(3))
            .expect("probe entered")
    }
}

impl Drop for HeldProbe {
    fn drop(&mut self) {
        self.release();
    }
}

fn hold_first_probe(cx: &mut TestAppContext, owner: &Entity<ClipboardApplication>) -> HeldProbe {
    let (entered, observed) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let blocked = Mutex::new(blocked);
    let first = AtomicBool::new(true);
    let timed_out = Arc::new(AtomicBool::new(false));
    let probe_timed_out = timed_out.clone();
    cx.update(|cx| {
        owner.update(cx, |owner, _| {
            owner.link_detection.resolver = Some(Arc::new(move |text| {
                entered.send(text.to_owned()).expect("record probe input");
                if first.swap(false, Ordering::SeqCst)
                    && blocked
                        .lock()
                        .expect("probe gate")
                        .recv_timeout(Duration::from_secs(3))
                        .is_err()
                {
                    probe_timed_out.store(true, Ordering::SeqCst);
                }
                detect_links(text)
            }));
        });
    });
    HeldProbe {
        entered: observed,
        release: Some(release),
        timed_out,
    }
}

fn visible_targets(cx: &mut TestAppContext, owner: &Entity<ClipboardApplication>) -> Vec<String> {
    let window = cx.read(|cx| owner.read(cx).link_panel.window);
    let Some(window) = window else {
        return Vec::new();
    };
    cx.update(|cx| {
        window
            .update(cx, |root, _, cx| {
                let root = root.downcast::<Root>().expect("link Root");
                let view = root
                    .read(cx)
                    .view()
                    .clone()
                    .downcast::<crate::tools::link_panel::LinkPanelView>()
                    .expect("link view");
                view.read(cx)
                    .links
                    .iter()
                    .map(|link| link.copy_value.clone())
                    .collect()
            })
            .expect("read link panel")
    })
}

#[gpui_kit::test]
fn link_detection_runs_filesystem_work_off_gpui(cx: &mut TestAppContext) {
    let (_directory, owner, worker, endpoint) = fixture(cx);
    let ui_thread = std::thread::current().id();
    let (probed, observed) = mpsc::channel();
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.link_detection.resolver = Some(Arc::new(move |_| {
                probed
                    .send(std::thread::current().id())
                    .expect("report probe thread");
                Vec::new()
            }));
            owner.handle_link_detector_capture(r"\\fixture\share\item.txt", cx);
        });
    });
    let probe_thread = observed.recv_timeout(Duration::from_secs(3));
    drain_detection(cx, &owner, &endpoint);
    worker
        .shutdown()
        .expect("stop link detection fixture worker");
    assert_ne!(probe_thread.expect("probe was executed"), ui_thread);
}

#[gpui_kit::test]
fn link_detection_keeps_one_worker_and_only_the_latest_waiting_capture(cx: &mut TestAppContext) {
    let (_directory, owner, worker, endpoint) = fixture(cx);
    let mut probe = hold_first_probe(cx, &owner);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_link_detector_capture("https://example.com/first", cx);
        })
    });
    assert_eq!(probe.next(), "https://example.com/first");
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_link_detector_capture("https://example.com/middle", cx);
            owner.handle_link_detector_capture("https://example.com/latest", cx);
            assert!(owner.link_detection.active.is_some());
            assert_eq!(
                owner
                    .link_detection
                    .queued
                    .as_ref()
                    .expect("latest queued")
                    .text,
                "https://example.com/latest"
            );
        })
    });
    assert!(
        probe.entered.try_recv().is_err(),
        "no replacement worker while probe is blocked"
    );
    assert!(
        !probe.timed_out.load(Ordering::SeqCst),
        "GPUI remained responsive during blocked work"
    );
    probe.release();
    drain_detection(cx, &owner, &endpoint);
    assert_eq!(probe.next(), "https://example.com/latest");
    assert!(probe.entered.try_recv().is_err());
    assert_eq!(
        visible_targets(cx, &owner),
        vec!["https://example.com/latest"]
    );
    worker.shutdown().expect("stop fixture worker");
}

#[gpui_kit::test]
fn link_detection_new_empty_capture_retires_pending_and_active_results(cx: &mut TestAppContext) {
    let (_directory, owner, worker, endpoint) = fixture(cx);
    let mut probe = hold_first_probe(cx, &owner);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_link_detector_capture("https://example.com/first", cx);
        })
    });
    probe.next();
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_link_detector_capture("https://example.com/queued", cx);
            owner.handle_platform_event(PlatformEvent::ClipboardCaptured(capture(None)), cx);
            assert!(owner.link_detection.queued.is_none());
        })
    });
    probe.release();
    drain_detection(cx, &owner, &endpoint);
    assert!(probe.entered.try_recv().is_err());
    assert!(visible_targets(cx, &owner).is_empty());
    assert!(!probe.timed_out.load(Ordering::SeqCst));
    worker.shutdown().expect("stop fixture worker");
}

#[gpui_kit::test]
fn link_detection_disable_pause_rules_and_disconnect_retire_inflight_results(
    cx: &mut TestAppContext,
) {
    for change in 0..4 {
        let (_directory, owner, worker, endpoint) = fixture(cx);
        let mut probe = hold_first_probe(cx, &owner);
        cx.update(|cx| {
            owner.update(cx, |owner, cx| {
                owner.handle_link_detector_capture("https://example.com/first", cx);
            })
        });
        probe.next();
        cx.update(|cx| {
            owner.update(cx, |owner, cx| {
                let old = owner.link_detection.active.expect("running detection");
                owner.handle_link_detector_capture("https://example.com/queued", cx);
                match change {
                    0 => {
                        owner.set_link_detector_enabled(false, cx);
                        owner.set_link_detector_enabled(true, cx);
                    }
                    1 => owner.set_capture_pause(true, false, cx),
                    2 => owner.apply_live_blacklist(
                        ProcessBlacklistSettings::default(),
                        uuid::Uuid::new_v4(),
                        cx,
                    ),
                    _ => owner.disconnect_capture_pause(cx),
                }
                assert!(owner.link_detection.queued.is_none());
                assert!(!owner.link_detection_is_current(old));
            })
        });
        probe.release();
        drain_detection(cx, &owner, &endpoint);
        assert!(probe.entered.try_recv().is_err());
        assert!(visible_targets(cx, &owner).is_empty());
        assert!(!probe.timed_out.load(Ordering::SeqCst));
        worker.shutdown().expect("stop fixture worker");
    }
}

#[gpui_kit::test]
fn link_detection_duplicate_delivery_preserves_the_current_request(cx: &mut TestAppContext) {
    let (_directory, owner, worker, endpoint) = fixture(cx);
    let mut probe = hold_first_probe(cx, &owner);
    let text = "https://example.com/duplicated";
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_platform_event(PlatformEvent::ClipboardCaptured(capture(Some(text))), cx);
        })
    });
    assert_eq!(probe.next(), text);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            let generation = owner.link_detection.generation;
            owner.handle_platform_event(PlatformEvent::ClipboardCaptured(capture(Some(text))), cx);
            assert_eq!(owner.link_detection.generation, generation);
            assert!(owner.link_detection.queued.is_none());
        })
    });
    probe.release();
    drain_detection(cx, &owner, &endpoint);
    assert!(probe.entered.try_recv().is_err());
    assert_eq!(visible_targets(cx, &owner), vec![text]);
    assert!(!probe.timed_out.load(Ordering::SeqCst));
    worker.shutdown().expect("stop duplicate fixture worker");
}
