//! Actual Settings hotkey reset gestures; temporary stores and fake IPC only.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext, size};
use nxtools_core::AppConfig;
use nxtools_ipc::{PlatformNotification, PlatformServerEndpoint};
use nxtools_protocol::Message;

struct Fixture {
    main: Entity<MainWindow>,
    clipboard: Entity<ClipboardApplication>,
    quick: Entity<QuickLaunchApplication>,
    window: AnyWindowHandle,
    endpoint: PlatformServerEndpoint,
    temp: tempfile::TempDir,
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.worker
            .clone()
            .shutdown()
            .expect("temporary worker shutdown");
    }
}
fn fixture(cx: &mut TestAppContext, custom: bool) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let temp = tempfile::tempdir().expect("isolated hotkey reset");
    let mut config = AppConfig::default();
    if custom {
        let binding = ClipboardHotKey::new(
            vec![
                nxtools_core::ClipboardHotKeyModifier::Control,
                nxtools_core::ClipboardHotKeyModifier::Alt,
            ],
            "V",
            "Ctrl+Alt+V",
        );
        config.clipboard.hotkey = binding.clone();
        config.quick_launch.hotkey = binding;
    } else {
        // Display copy/case is not binding identity.
        config.clipboard.hotkey.display = "Alt + c".into();
        config.clipboard.hotkey.key = "c".into();
        config.quick_launch.hotkey.display = "Ctrl × 2".into();
    }
    let config_path = temp.path().join("config.json");
    config.save_to(&config_path).expect("seed temporary config");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        temp.path().join("clipboard.db"),
        temp.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("isolated history worker");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let platform = Rc::new(platform);
    let notices = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let clipboard = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                platform.clone(),
                config.clone(),
                config_path.clone(),
                notices.clone(),
                cx,
            )
        })
    });
    let catalog = nxtools_finder::FinderCatalog::open(temp.path().join("finder.db"), &[])
        .expect("empty finder catalog");
    let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
    let quick = cx.update(|cx| {
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                platform,
                Rc::new(indexer),
                config,
                config_path,
                temp.path().join("logs"),
                notices.clone(),
            )
        })
    });
    quick.update(cx, |app, cx| {
        app.handle_platform_notification(PlatformNotification::Connected, cx)
    });
    let initial = endpoint
        .try_receive_command_for_contract_test()
        .expect("initial quick configuration");
    assert!(matches!(
        initial.message,
        Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(_))
    ));
    quick.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: initial.id,
                result: result(true),
            },
            cx,
        )
    });
    // Constructor/Connected may enqueue unrelated fake bootstrap messages. Only
    // measure commands caused by gestures after this initialization boundary.
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let clip = clipboard.downgrade();
    let launch = quick.downgrade();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                clip,
                launch,
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notices,
                window,
                cx,
            );
            main.shell.activate(MainModule::Clipboard);
            main
        });
        *slot.borrow_mut() = Some(main.clone());
        Root::new(main, window, cx)
    });
    surface.simulate_resize(size(px(920.0), px(650.0)));
    surface.run_until_parked();
    let window = surface.update(|window, _| window.window_handle());
    let main = retained.borrow_mut().take().expect("original main owner");
    Fixture {
        main,
        clipboard,
        quick,
        window,
        endpoint,
        temp,
        worker,
    }
}
fn result(success: bool) -> nxtools_protocol::PlatformCommandResult {
    nxtools_protocol::PlatformCommandResult {
        success,
        error: (!success).then(|| "synthetic conflict".into()),
        native_error_code: (!success).then_some(1409),
        data: None,
    }
}
fn terminal(
    f: &Fixture,
    target: MainModule,
    id: uuid::Uuid,
    success: bool,
    cx: &mut TestAppContext,
) {
    let notification = PlatformNotification::CommandResult {
        correlation_id: id,
        result: result(success),
    };
    if target == MainModule::Clipboard {
        f.clipboard.update(cx, |app, cx| {
            app.handle_platform_notification(notification, cx)
        });
    } else {
        f.quick.update(cx, |app, cx| {
            app.handle_platform_notification(notification, cx)
        });
    }
    cx.run_until_parked();
}
fn reset_id(target: MainModule) -> &'static str {
    if target == MainModule::Clipboard {
        "clipboard-hotkey-reset"
    } else {
        "quick-launch-hotkey-reset"
    }
}
fn button(surface: &mut VisualTestContext, id: &str) -> Option<test_support::ElementSnapshot> {
    surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&gpui_kit::ElementId::from(id.to_owned())))
    })
}
fn show(f: &Fixture, surface: &mut VisualTestContext, target: MainModule) {
    f.main.update(surface, |main, cx| {
        main.shell.activate(target);
        cx.notify();
    });
    surface.run_until_parked();
    surface.refresh().expect("actual hotkey settings page");
}
fn configured(f: &Fixture, target: MainModule, surface: &VisualTestContext) -> ClipboardHotKey {
    if target == MainModule::Clipboard {
        f.clipboard
            .read_with(surface, |app, _| app.clipboard_settings().hotkey)
    } else {
        f.quick.read_with(surface, |app, _| app.settings().hotkey)
    }
}

#[gpui_kit::test]
fn hotkey_reset_defaults_are_hidden_even_with_different_display_copy(cx: &mut TestAppContext) {
    let f = fixture(cx, false);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    for target in [MainModule::Clipboard, MainModule::QuickFind] {
        show(&f, &mut surface, target);
        assert!(
            button(&mut surface, reset_id(target)).is_none(),
            "default shortcut has no reset action"
        );
    }
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn hotkey_reset_actual_pages_keep_left_placement_ack_save_and_retry(cx: &mut TestAppContext) {
    let f = fixture(cx, true);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    for target in [MainModule::Clipboard, MainModule::QuickFind] {
        show(&f, &mut surface, target);
        let original = configured(&f, target, &surface);
        let default = if target == MainModule::Clipboard {
            AppConfig::default().clipboard.hotkey
        } else {
            AppConfig::default().quick_launch.hotkey
        };
        for success in [false, true] {
            surface.refresh().expect("retryable reset control");
            let reset =
                button(&mut surface, reset_id(target)).expect("nondefault hotkey reset Button");
            assert_eq!(reset.label(), Some("重置"));
            assert_eq!(
                reset.disabled(),
                None,
                "reset eligible: {target:?}, keyboard={success}"
            );
            let recorder_id = if target == MainModule::Clipboard {
                "record-clipboard-hotkey"
            } else {
                "quick-launch-hotkey-field-action"
            };
            let recorder = button(&mut surface, recorder_id).expect("original recorder Button");
            assert!(
                reset.bounds().right() < recorder.bounds().left(),
                "reset precedes shortcut"
            );
            assert!((reset.bounds().center().y - recorder.bounds().center().y).abs() <= px(1.0));
            assert_eq!(reset.bounds().size.height, px(32.0));
            if success {
                // Actual official Button owns focus and Enter release, not a custom key dispatcher.
                surface.update(|window, _| window.activate_window());
                for _ in 0..30 {
                    surface.update(|window, cx| window.focus_next(cx));
                    surface.refresh().expect("Tab to reset");
                    if button(&mut surface, reset_id(target))
                        .is_some_and(|node| node.focused() == Some(true))
                    {
                        break;
                    }
                }
                assert_eq!(
                    button(&mut surface, reset_id(target))
                        .expect("focused reset")
                        .focused(),
                    Some(true)
                );
                let keystroke = gpui_kit::Keystroke::parse("enter").expect("reset Enter");
                surface.simulate_event(gpui_kit::KeyDownEvent {
                    keystroke: keystroke.clone(),
                    is_held: false,
                    prefer_character_input: false,
                });
                assert!(
                    f.endpoint.try_receive_command_for_contract_test().is_err(),
                    "not before key release"
                );
                surface.simulate_event(gpui_kit::KeyUpEvent { keystroke });
            } else {
                surface.simulate_click(reset.bounds().center(), Modifiers::default());
            }
            surface.run_until_parked();
            let command = f
                .endpoint
                .try_receive_command_for_contract_test()
                .unwrap_or_else(|error| {
                    panic!(
                        "one default configuration: target={target:?}, keyboard={success}: {error}"
                    )
                });
            match &command.message {
                Message::PlatformCommand(PlatformCommand::ConfigureClipboard(command))
                    if target == MainModule::Clipboard =>
                {
                    assert_eq!(command.hotkey_key, default.key);
                    assert_eq!(command.hotkey_modifiers, ["alt"]);
                }
                Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(command))
                    if target == MainModule::QuickFind =>
                {
                    assert_eq!(command.hotkey_key, default.key);
                    assert!(command.hotkey_modifiers.is_empty());
                }
                _ => panic!("reset must configure directly, never simulate recording"),
            }
            surface.refresh().expect("pending reset");
            let pending =
                button(&mut surface, reset_id(target)).expect("pending reset remains discoverable");
            assert_eq!(pending.disabled(), Some(true));
            surface.simulate_click(pending.bounds().center(), Modifiers::default());
            surface.run_until_parked();
            assert!(
                f.endpoint.try_receive_command_for_contract_test().is_err(),
                "pending rejects repeat"
            );
            assert_eq!(
                configured(&f, target, &surface),
                original,
                "no optimistic binding publication"
            );
            let before =
                AppConfig::load_from(&f.temp.path().join("config.json")).expect("preACK disk");
            assert_eq!(
                if target == MainModule::Clipboard {
                    before.clipboard.hotkey
                } else {
                    before.quick_launch.hotkey
                },
                original
            );
            terminal(&f, target, uuid::Uuid::new_v4(), true, cx);
            assert_eq!(
                configured(&f, target, &surface),
                original,
                "unmatched ACK ignored"
            );
            terminal(&f, target, command.id, success, cx);
            surface.refresh().expect("terminal reset");
            let expected = if success { &default } else { &original };
            assert_eq!(&configured(&f, target, &surface), expected);
            let disk =
                AppConfig::load_from(&f.temp.path().join("config.json")).expect("terminal disk");
            assert_eq!(
                &(if target == MainModule::Clipboard {
                    disk.clipboard.hotkey
                } else {
                    disk.quick_launch.hotkey
                }),
                expected
            );
            assert_eq!(
                button(&mut surface, reset_id(target)).is_none(),
                success,
                "hide only after default applied"
            );
        }
    }
}

#[gpui_kit::test]
fn hotkey_reset_recording_and_disconnect_reject_both_ui_and_owner(cx: &mut TestAppContext) {
    let f = fixture(cx, true);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    for target in [MainModule::Clipboard, MainModule::QuickFind] {
        show(&f, &mut surface, target);
        if target == MainModule::Clipboard {
            f.clipboard.update(&mut surface, |app, _| {
                app.set_clipboard_hotkey_recording(true)
            });
        } else {
            f.quick
                .update(&mut surface, |app, _| app.set_hotkey_recording(true));
        }
        let request = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("recording pause request");
        for ready in [false, true] {
            if ready {
                terminal(&f, target, request.id, true, cx);
            }
            surface.refresh().expect("recording reset disabled");
            let reset =
                button(&mut surface, reset_id(target)).expect("changed reset still discoverable");
            assert_eq!(reset.disabled(), Some(true));
            surface.simulate_click(reset.bounds().center(), Modifiers::default());
            if target == MainModule::Clipboard {
                f.clipboard
                    .update(&mut surface, |app, _| app.reset_clipboard_hotkey());
            } else {
                f.quick.update(&mut surface, |app, _| app.reset_hotkey());
            }
            surface.run_until_parked();
            assert!(
                f.endpoint.try_receive_command_for_contract_test().is_err(),
                "recording owner rejects reset"
            );
        }
        if target == MainModule::Clipboard {
            f.clipboard.update(&mut surface, |app, _| {
                app.set_clipboard_hotkey_recording(false)
            });
        } else {
            f.quick
                .update(&mut surface, |app, _| app.set_hotkey_recording(false));
        }
        let cancel = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("original recording cancel");
        terminal(&f, target, cancel.id, true, cx);
        surface.refresh().expect("recording released");
        assert_eq!(
            button(&mut surface, reset_id(target))
                .expect("reset restored")
                .disabled(),
            None
        );
    }
    f.endpoint.set_connected_for_contract_test(false);
    for target in [MainModule::Clipboard, MainModule::QuickFind] {
        show(&f, &mut surface, target);
        let reset = button(&mut surface, reset_id(target)).expect("disconnected reset");
        assert_eq!(reset.disabled(), Some(true));
        surface.simulate_click(reset.bounds().center(), Modifiers::default());
        if target == MainModule::Clipboard {
            f.clipboard
                .update(&mut surface, |app, _| app.reset_clipboard_hotkey());
        } else {
            f.quick.update(&mut surface, |app, _| app.reset_hotkey());
        }
        surface.run_until_parked();
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    }
}
