use super::*;
use crate::clipboard_worker::ClipboardCaptureWorkerHandle;
use gpui_kit::TestAppContext;

fn fixture(
    cx: &mut TestAppContext,
) -> (
    tempfile::TempDir,
    Entity<ClipboardApplication>,
    Entity<NotificationCenter>,
    ClipboardCaptureWorkerHandle,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let dir = tempfile::tempdir().expect("currency fixture");
    let worker = ClipboardCaptureWorkerHandle::spawn(
        dir.path().join("history.db"),
        dir.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 100,
            retention_days: 30,
        },
        1_000,
    )
    .expect("worker");
    let (platform, _endpoint) = nxtools_ipc::platform_bridge(64);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let mut config = AppConfig::default();
    config.clipboard.currency_conversion.enabled = true;
    let path = dir.path().join("config.json");
    config.save_to(&path).expect("config");
    let owner = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                config,
                path,
                notifications.clone(),
                cx,
            )
        })
    });
    let now = nxtools_ipc::unix_time_ms();
    cx.update(|cx| {
        owner.update(cx, |owner, _| {
            owner.currency.rates = Some(RateSnapshot {
                version: 2,
                provider: "fawazahmed0 Currency API".into(),
                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                fetched_at: now,
                names: [
                    ("CNY".into(), "Chinese Yuan".into()),
                    ("USD".into(), "US Dollar".into()),
                    ("CAD".into(), "Canadian Dollar".into()),
                    ("EUR".into(), "Euro".into()),
                    ("KRW".into(), "South Korean Won".into()),
                ]
                .into(),
                rates: [
                    ("CNY".into(), 1.0),
                    ("USD".into(), 0.14),
                    ("CAD".into(), 0.2),
                    ("EUR".into(), 0.125),
                    ("KRW".into(), 200.0),
                ]
                .into(),
            });
        })
    });
    (dir, owner, notifications, worker)
}
fn capture(text: &str) -> ClipboardCapture {
    ClipboardCapture {
        text: Some(text.into()),
        image: None,
        paths: Vec::new(),
        native_formats: Vec::new(),
        source_icon_staging_path: None,
        source: nxtools_protocol::ForegroundContext {
            window_handle: 0,
            focus_window_handle: 0,
            process_id: 1,
            process_name: "fixture.exe".into(),
            window_title: "fixture".into(),
        },
        captured_at_unix_ms: nxtools_ipc::unix_time_ms(),
    }
}
#[gpui_kit::test]
fn currency_tool_capture_uses_shared_notification_and_privacy_guards(cx: &mut TestAppContext) {
    let (_dir, owner, notifications, worker) = fixture(cx);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_platform_event(
                PlatformEvent::ClipboardCaptured(capture("5CAD 2.5cad 32$ €43")),
                cx,
            )
        })
    });
    cx.run_until_parked();
    cx.read(|cx| {
        let notices = notifications.read(cx).items();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].message.contains("25.00 CNY"));
        assert!(notices[0].message.contains("43 EUR"));
    });
    for mode in 0..4 {
        cx.update(|cx| {
            owner.update(cx, |owner, cx| {
                match mode {
                    0 => owner.config.clipboard.currency_conversion.enabled = false,
                    1 => {
                        owner.config.clipboard.currency_conversion.enabled = true;
                        owner
                            .config
                            .set_process_blacklist(vec!["fixture.exe".into()], true, false);
                    }
                    2 => {
                        owner.config.set_process_blacklist(Vec::new(), true, false);
                        owner.arm_capture_suppression(ClipboardAction::CopyOnly);
                    }
                    _ => {
                        owner.set_capture_pause(true, false, cx);
                    }
                }
                owner.handle_platform_event(PlatformEvent::ClipboardCaptured(capture("5CAD")), cx);
            })
        });
        cx.run_until_parked();
        cx.read(|cx| assert_eq!(notifications.read(cx).items().len(), 1));
    }
    worker.shutdown().expect("shutdown");
}
#[gpui_kit::test]
fn currency_tool_deferred_notices_and_refresh_reject_stale_owners(cx: &mut TestAppContext) {
    let (_dir, owner, notifications, worker) = fixture(cx);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_currency_capture("5CAD", cx);
            owner.invalidate_currency_capture();
        })
    });
    cx.run_until_parked();
    cx.read(|cx| assert!(notifications.read(cx).items().is_empty()));
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            let old = owner.currency.rates.clone().expect("last good");
            let id = uuid::Uuid::new_v4();
            owner.currency.pending = Some(id);
            owner.finish_currency_refresh(uuid::Uuid::new_v4(), Err("stale"), Instant::now(), cx);
            assert_eq!(owner.currency.pending, Some(id));
            owner.finish_currency_refresh(id, Err("request"), Instant::now(), cx);
            assert!(owner.currency.pending.is_none());
            assert_eq!(
                owner.currency.rates.as_ref().expect("preserved").rates,
                old.rates
            );
            assert!(owner.currency_status().error.is_some());
            owner.currency.rates = None;
            owner.currency.pending = Some(id);
            owner.handle_currency_capture("32$", cx);
            assert!(owner.currency.queued.is_some());
            owner.set_currency_settings(false, "CAD".into(), cx);
            owner.finish_currency_refresh(id, Ok((old, true)), Instant::now(), cx);
        })
    });
    cx.run_until_parked();
    cx.read(|cx| assert!(notifications.read(cx).items().is_empty()));
    worker.shutdown().expect("shutdown");
}

#[gpui_kit::test]
fn currency_tool_singleflight_daily_owner_and_failed_save_preserve_state(cx: &mut TestAppContext) {
    let (dir, owner, notifications, worker) = fixture(cx);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            let id = uuid::Uuid::new_v4();
            owner.currency.pending = Some(id);
            owner.refresh_currency_rates(true, cx);
            assert_eq!(owner.currency.pending, Some(id), "one refresh");
            owner.currency.pending = None;
            owner.refresh_currency_rates(false, cx);
            assert!(
                owner.currency.pending.is_none(),
                "fresh snapshot no request"
            );
            owner.schedule_currency_daily(cx);
            assert!(owner.currency.daily_task.is_some());
            owner.set_currency_settings(false, "CNY".into(), cx);
            assert!(
                owner.currency.daily_task.is_none(),
                "disable cancels daily timer"
            );
            let config_path = dir.path().join("config.json");
            std::fs::write(&config_path, b"corrupt").expect("failed save fixture");
            owner.set_currency_settings(true, "CAD".into(), cx);
            assert!(!owner.currency_status().enabled);
            assert_eq!(owner.currency_status().target, "CNY");
            assert!(owner.currency_status().error.is_some());
            assert_eq!(
                std::fs::read(config_path).expect("no overwrite"),
                b"corrupt"
            );
        })
    });
    cx.run_until_parked();
    cx.read(|cx| assert!(notifications.read(cx).items().is_empty()));
    worker.shutdown().expect("shutdown");
}

#[gpui_kit::test]
fn currency_catalog_clipboard_won_and_dollar_aliases_share_one_local_notice(
    cx: &mut TestAppContext,
) {
    let (_dir, owner, notifications, worker) = fixture(cx);
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            owner.handle_platform_event(
                PlatformEvent::ClipboardCaptured(capture("₩1000 5美刀 2刀")),
                cx,
            )
        })
    });
    cx.run_until_parked();
    cx.read(|cx| {
        let items = notifications.read(cx).items();
        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("1000 KRW ≈ 5.00 CNY"));
        assert!(items[0].message.contains("5 USD"));
        assert!(items[0].message.contains("2 USD"));
    });
    worker.shutdown().expect("shutdown");
}
