use super::*;
use gpui_kit::{Render, TestAppContext, VisualTestContext, Window, div};
use nxtools_protocol::{Envelope, Message, PlatformCommandResult};

struct MainFixture;
impl Render for MainFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui_kit::IntoElement {
        div()
    }
}
fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<FloatingAssistantApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    tempfile::TempDir,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_, window) = cx.add_window_view(|window, cx| {
        gpui_kit::base::Root::new(cx.new(|_| MainFixture), window, cx)
    });
    let source = window.update(|window, _| window.window_handle());
    let directory = tempfile::tempdir().expect("temporary config");
    let path = directory.path().join("config.json");
    let mut config = AppConfig::default();
    config.floating_assistant.enabled = true;
    config.save_to(&path).expect("write temporary config");
    let center = cx.new(|_| NotificationCenter::default());
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(64);
    endpoint.set_connected_for_contract_test(true);
    let owner = cx.new(|cx| {
        FloatingAssistantApplication::new(
            Rc::new(bridge),
            config,
            path,
            center,
            source,
            WeakEntity::new_invalid(),
            WeakEntity::new_invalid(),
            cx,
        )
    });
    owner.update(cx, |owner, _| owner.source_hwnd = -39);
    (owner, endpoint, directory)
}
fn next(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Envelope {
    endpoint
        .try_receive_command_for_contract_test()
        .expect("native command")
}
fn operation(envelope: &Envelope) -> FloatingAssistantOperation {
    let Message::PlatformCommand(PlatformCommand::FloatingAssistant(command)) = &envelope.message
    else {
        panic!("floating command");
    };
    command.operation
}
fn reply(
    owner: &Entity<FloatingAssistantApplication>,
    id: Uuid,
    success: bool,
    cx: &mut TestAppContext,
) {
    if success
        && owner.read_with(cx, |owner, _| {
            owner
                .lifecycle
                .pending
                .is_some_and(|(_, phase)| matches!(phase, Phase::Prepare | Phase::Update))
        })
    {
        let handle = owner.read_with(cx, |owner, _| {
            owner.surface.as_ref().expect("surface").window
        });
        let window = VisualTestContext::from_window(handle, cx);
        window.simulate_scale_factor_change(1.0);
        window.simulate_resize(size(px(72.0), px(72.0)));
    }
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::CommandResult {
                correlation_id: id,
                result: PlatformCommandResult {
                    success,
                    error: (!success).then(|| "synthetic_native_failure".into()),
                    native_error_code: None,
                    data: Some(
                        serde_json::to_string(&FloatingAssistantBounds {
                            x: 40,
                            y: 180,
                            width: 72,
                            height: 72,
                            dpi: 96,
                            work_left: 0,
                            work_top: 0,
                            work_right: 1920,
                            work_bottom: 1040,
                            monitor_left: 0,
                            monitor_top: 0,
                            monitor_right: 1920,
                            monitor_bottom: 1080,
                        })
                        .expect("readback"),
                    ),
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
}
#[gpui_kit::test]
fn floating_assistant_native_pending_precedes_prepare_and_frame_precedes_reveal(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    assert_eq!(operation(&prepare), FloatingAssistantOperation::Prepare);
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.lifecycle.pending),
        Some((prepare.id, Phase::Prepare))
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    assert_eq!(operation(&show), FloatingAssistantOperation::Show);
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
    reply(&owner, show.id, true, cx);
    assert!(owner.read_with(cx, |owner, _| owner.lifecycle.visible));
    let handle = owner.read_with(cx, |owner, _| {
        owner.surface.as_ref().expect("surface").window
    });
    let mut window = VisualTestContext::from_window(handle, cx);
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.debug_bounds("floating-assistant-ball").is_some());
}
#[gpui_kit::test]
fn floating_assistant_disable_during_prepare_cannot_reveal_and_reuses_window(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    let handle = owner.read_with(cx, |owner, _| {
        owner.surface.as_ref().expect("surface").window
    });
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.enabled = false;
        owner.change_settings(settings, cx);
    });
    reply(&owner, prepare.id, true, cx);
    let hide = next(&endpoint);
    assert_eq!(operation(&hide), FloatingAssistantOperation::Hide);
    reply(&owner, hide.id, true, cx);
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
    assert_eq!(
        owner.read_with(cx, |owner, _| owner
            .surface
            .as_ref()
            .expect("same surface")
            .window),
        handle
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
#[gpui_kit::test]
fn floating_assistant_retry_waits_for_retirement_before_repreparing(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let old = next(&endpoint);
    reply(&owner, old.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    owner.update(cx, |owner, cx| owner.retry(cx));
    let retire = next(&endpoint);
    assert_eq!(operation(&retire), FloatingAssistantOperation::Retire);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "new prepare cannot overtake native retirement admission"
    );
    reply(&owner, retire.id, true, cx);
    let replacement = next(&endpoint);
    assert_eq!(operation(&replacement), FloatingAssistantOperation::Prepare);
    reply(&owner, old.id, true, cx);
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.lifecycle.pending),
        Some((replacement.id, Phase::Prepare))
    );
}
#[gpui_kit::test]
fn floating_assistant_disconnect_and_late_ack_never_reveal(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let old = next(&endpoint);
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![old.id],
            },
            cx,
        )
    });
    reply(&owner, old.id, true, cx);
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn floating_assistant_native_failure_stops_actual_gif_task(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    cx.update(|cx| {
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: false,
        })
    });
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let view = owner.read_with(cx, |owner, _| {
        owner.surface.as_ref().expect("surface").view.clone()
    });
    view.update(cx, |view, cx| {
        view.replace(
            Some(media::DecodedMedia {
                animated: true,
                frames: vec![
                    media::MediaFrame {
                        width: 1,
                        height: 1,
                        rgba: vec![255, 0, 0, 255],
                        duration: Duration::from_millis(100),
                    },
                    media::MediaFrame {
                        width: 1,
                        height: 1,
                        rgba: vec![0, 255, 0, 255],
                        duration: Duration::from_millis(100),
                    },
                ],
            }),
            24,
            cx,
        )
    });
    assert!(view.read_with(cx, |view, _| view.timer_active()));
    owner.update(cx, |owner, cx| {
        owner.dirty = true;
        owner.sync(cx);
    });
    let update = next(&endpoint);
    reply(&owner, update.id, false, cx);
    assert!(
        !view.read_with(cx, |view, _| view.timer_active()),
        "native failure must cancel real playback task"
    );
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::Hide
    );
}

#[gpui_kit::test]
fn floating_assistant_failed_save_preserves_live_and_disk_config(cx: &mut TestAppContext) {
    let (owner, endpoint, directory) = fixture(cx);
    let path = directory.path().join("config.json");
    std::fs::write(&path, b"{corrupt").expect("corrupt temporary config");
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.size = 96;
        owner.change_settings(settings, cx);
    });
    assert_eq!(owner.read_with(cx, |owner, _| owner.settings.size), 72);
    assert_eq!(std::fs::read(&path).expect("read unchanged"), b"{corrupt");
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(owner.read_with(cx, |owner, _| owner.error.is_some()));
}

fn drop_event(
    owner: &Entity<FloatingAssistantApplication>,
    cx: &TestAppContext,
    path: Option<String>,
    sequence: u64,
    id: Uuid,
) -> FloatingAssistantEvent {
    owner.read_with(cx, |owner, _| FloatingAssistantEvent {
        version: 1,
        session_id: owner.session,
        generation: owner.generation,
        window_handle: owner.surface.as_ref().expect("surface").hwnd,
        correlation_id: id,
        sequence,
        kind: FloatingAssistantEventKind::Drop,
        x: 40,
        y: 180,
        delta: 0,
        fullscreen: false,
        text: None,
        url: None,
        html: None,
        image_path: path,
        image_format: Some("png".into()),
        files: vec![],
        drop_id: Some(id),
    })
}
#[gpui_kit::test]
fn floating_assistant_drop_copy_is_once_only_copy_command_and_releases_after_result(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let path = directory.path().join("source.png");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([12, 34, 56, 255]))
        .save(&path)
        .expect("synthetic PNG");
    let id = Uuid::new_v4();
    let event = drop_event(&owner, cx, Some(path.to_string_lossy().into_owned()), 1, id);
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event.clone())),
            cx,
        )
    });
    let mut command = None;
    for _ in 0..200 {
        cx.run_until_parked();
        if let Ok(value) = endpoint.try_receive_command_for_contract_test() {
            command = Some(value);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let command = command.expect("prepared image copy");
    let Message::PlatformCommand(PlatformCommand::PasteImage(copy)) = &command.message else {
        panic!("copy-only image command");
    };
    assert!(!copy.paste_after_write && !copy.include_file_drop_fallback);
    assert_eq!(command.id, id);
    let staged = PathBuf::from(&copy.image_path);
    assert!(staged.is_file());
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.pending_actions.len()),
        1
    );
    owner.update(cx, |owner, cx| {
        let mut duplicate = event.clone();
        duplicate.sequence = 2;
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(duplicate)),
            cx,
        );
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "duplicate drop cannot dispatch or release active data"
    );
    reply(&owner, command.id, true, cx);
    let released = next(&endpoint);
    assert_eq!(
        operation(&released),
        FloatingAssistantOperation::ReleaseDrop
    );
    assert!(!staged.exists());
    assert!(owner.read_with(cx, |owner, _| owner.pending_actions.is_empty()));
}

#[gpui_kit::test]
fn floating_assistant_cancelled_deferred_routes_release_drop_without_dispatch(
    cx: &mut TestAppContext,
) {
    for text in ["https://example.test/file.zip", "完整 搜索 文本"] {
        let (owner, endpoint, _directory) = fixture(cx);
        owner.update(cx, |owner, cx| owner.start(cx));
        let prepare = next(&endpoint);
        reply(&owner, prepare.id, true, cx);
        let show = next(&endpoint);
        reply(&owner, show.id, true, cx);
        let id = Uuid::new_v4();
        let mut event = drop_event(&owner, cx, None, 1, id);
        event.image_format = None;
        event.text = Some(text.into());
        owner.update(cx, |owner, cx| {
            owner.handle(
                PlatformNotification::Event(PlatformEvent::FloatingAssistant(event)),
                cx,
            );
            let mut settings = owner.settings.clone();
            settings.enabled = false;
            owner.change_settings(settings, cx);
        });
        cx.run_until_parked();
        let mut releases = 0;
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            match operation(&envelope) {
                FloatingAssistantOperation::ReleaseDrop => releases += 1,
                FloatingAssistantOperation::Hide => {}
                other => panic!("cancelled action emitted {other:?}"),
            }
        }
        assert_eq!(
            releases, 1,
            "cancelled deferred route releases exact staging once"
        );
        assert!(owner.read_with(cx, |owner, _| owner.pending_actions.is_empty()));
    }
}

#[gpui_kit::test]
fn floating_assistant_disable_keeps_dispatched_copy_alive_until_native_result(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let path = directory.path().join("source.png");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([12, 34, 56, 255]))
        .save(&path)
        .expect("synthetic PNG");
    let id = Uuid::new_v4();
    let event = drop_event(&owner, cx, Some(path.to_string_lossy().into_owned()), 1, id);
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event)),
            cx,
        )
    });
    let mut command = None;
    for _ in 0..200 {
        cx.run_until_parked();
        if let Ok(value) = endpoint.try_receive_command_for_contract_test() {
            command = Some(value);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let command = command.expect("prepared copy");
    let Message::PlatformCommand(PlatformCommand::PasteImage(copy)) = &command.message else {
        panic!("copy command");
    };
    let staged = PathBuf::from(&copy.image_path);
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.enabled = false;
        owner.change_settings(settings, cx);
    });
    cx.run_until_parked();
    assert!(
        staged.is_file(),
        "an admitted native command still owns the image until its result"
    );
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::Hide
    );
    reply(&owner, id, true, cx);
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::ReleaseDrop
    );
    assert!(!staged.exists());
    assert!(owner.read_with(cx, |owner, _| owner.pending_actions.is_empty()));
}

#[gpui_kit::test]
fn floating_assistant_retry_cancels_deferred_route_before_retirement(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let id = Uuid::new_v4();
    let mut event = drop_event(&owner, cx, None, 1, id);
    event.image_format = None;
    event.text = Some("完整 搜索 文本".into());
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event)),
            cx,
        );
        owner.retry(cx);
    });
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::ReleaseDrop
    );
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::Retire
    );
    cx.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(owner.read_with(cx, |owner, _| owner.pending_actions.is_empty()));
}

#[gpui_kit::test]
fn floating_assistant_main_restore_is_bounded_and_activation_follows_success(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let id = Uuid::new_v4();
    let mut event = drop_event(&owner, cx, None, 1, id);
    event.kind = FloatingAssistantEventKind::LeftClick;
    event.drop_id = None;
    event.image_format = None;
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event.clone())),
            cx,
        )
    });
    let restore = next(&endpoint);
    assert!(
        matches!(restore.message, Message::PlatformCommand(PlatformCommand::SetWindowVisibility(ref command)) if command.visible)
    );
    assert_eq!(restore.correlation_id, Some(id));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    for sequence in 2..12 {
        event.sequence = sequence;
        event.correlation_id = Uuid::new_v4();
        owner.update(cx, |owner, cx| {
            owner.handle(
                PlatformNotification::Event(PlatformEvent::FloatingAssistant(event.clone())),
                cx,
            )
        });
    }
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.pending_actions.len()),
        1
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    reply(&owner, restore.id, false, cx);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "failed restore cannot activate"
    );
    event.sequence = 12;
    event.correlation_id = Uuid::new_v4();
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event.clone())),
            cx,
        )
    });
    let restore = next(&endpoint);
    reply(&owner, restore.id, true, cx);
    let activate = next(&endpoint);
    assert!(matches!(
        activate.message,
        Message::PlatformCommand(PlatformCommand::ActivateWindow(_))
    ));
    assert_eq!(activate.correlation_id, restore.correlation_id);
    reply(&owner, activate.id, true, cx);
    assert!(owner.read_with(cx, |owner, _| owner.pending_actions.is_empty()));
}

#[gpui_kit::test]
fn floating_assistant_unknown_download_progress_keeps_visible_count(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let handle = owner.read_with(cx, |owner, _| {
        owner.surface.as_ref().expect("surface").window
    });
    owner.update(cx, |owner, cx| {
        owner.download_projection.active_count = 3;
        owner.download_projection.progress = None;
        owner.update_view(cx);
    });
    cx.run_until_parked();
    let mut window = VisualTestContext::from_window(handle, cx);
    window.refresh().expect("download count");
    assert!(window.debug_bounds("floating-download-count").is_some());
    assert!(window.debug_bounds("floating-download-ring").is_none());
    owner.update(cx, |owner, cx| {
        owner.download_projection.progress = Some(25.0);
        owner.update_view(cx);
    });
    cx.run_until_parked();
    window.refresh().expect("known progress");
    let ring = window
        .debug_bounds("floating-download-ring")
        .expect("official progress");
    assert_eq!(ring.size, size(px(72.0), px(72.0)));
}

#[gpui_kit::test]
fn floating_assistant_environment_reapplies_revealed_bounds_and_fullscreen_recovery(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let mut event = drop_event(&owner, cx, None, 1, Uuid::new_v4());
    event.kind = FloatingAssistantEventKind::Environment;
    event.drop_id = None;
    event.image_format = None;
    owner.update(cx, |owner, cx| {
        owner.edge_hidden = true;
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event.clone())),
            cx,
        );
    });
    let update = next(&endpoint);
    let Message::PlatformCommand(PlatformCommand::FloatingAssistant(command)) = &update.message
    else {
        panic!("native update");
    };
    assert_eq!(command.operation, FloatingAssistantOperation::Update);
    assert!(
        !command.edge_hidden,
        "display recovery first restores a fully visible anchor"
    );
    reply(&owner, update.id, true, cx);
    event.sequence = 2;
    event.fullscreen = true;
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event.clone())),
            cx,
        )
    });
    let hide = next(&endpoint);
    assert_eq!(operation(&hide), FloatingAssistantOperation::Hide);
    reply(&owner, hide.id, true, cx);
    event.sequence = 3;
    event.fullscreen = false;
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event)),
            cx,
        )
    });
    let update = next(&endpoint);
    assert_eq!(operation(&update), FloatingAssistantOperation::Update);
    reply(&owner, update.id, true, cx);
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::Show
    );
}

#[gpui_kit::test]
fn floating_assistant_disable_during_image_preparation_never_dispatches_late_copy(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let path = directory.path().join("source.png");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
        .save(&path)
        .expect("synthetic PNG");
    let event = drop_event(
        &owner,
        cx,
        Some(path.to_string_lossy().into_owned()),
        1,
        Uuid::new_v4(),
    );
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event)),
            cx,
        );
        let mut settings = owner.settings.clone();
        settings.enabled = false;
        owner.change_settings(settings, cx);
    });
    for _ in 0..30 {
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut releases = 0;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        match operation(&envelope) {
            FloatingAssistantOperation::ReleaseDrop => releases += 1,
            FloatingAssistantOperation::Hide => {}
            other => panic!("late image preparation emitted {other:?}"),
        }
    }
    assert_eq!(releases, 1);
    assert!(owner.read_with(cx, |owner, _| owner.pending_actions.is_empty()));
}

#[gpui_kit::test]
fn floating_assistant_disable_invalidates_decode_cache_and_reenable_loads_current_image(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let path = directory.path().join("current.png");
    image::RgbaImage::from_pixel(1, 1, image::Rgba([8, 9, 10, 255]))
        .save(&path)
        .expect("synthetic PNG");
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.image_path = Some(path.to_string_lossy().into_owned());
        owner.change_settings(settings, cx);
        let mut settings = owner.settings.clone();
        settings.enabled = false;
        owner.change_settings(settings, cx);
    });
    assert!(
        owner.read_with(cx, |owner, _| owner.loaded_asset.is_none()),
        "a cancelled decode cannot remain a loaded-cache hit"
    );
    let update = next(&endpoint);
    reply(&owner, update.id, true, cx);
    let hide = next(&endpoint);
    reply(&owner, hide.id, true, cx);
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.enabled = true;
        owner.change_settings(settings, cx);
    });
    let view = owner.read_with(cx, |owner, _| {
        owner.surface.as_ref().expect("surface").view.clone()
    });
    for _ in 0..200 {
        cx.run_until_parked();
        if view.read_with(cx, |view, _| view.image_count()) == 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.image_count()),
        1,
        "current image must be decoded after reenable"
    );
}

#[gpui_kit::test]
fn runtime_feedback_picker_wait_has_no_false_timeout_and_cancel_is_quiet(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.pick_image(cx));
    let request = next(&endpoint);
    cx.executor().advance_clock(Duration::from_secs(601));
    cx.run_until_parked();
    owner.read_with(cx, |owner, _| {
        assert!(owner.picker.is_some(), "waiting user must not time out");
        assert!(owner.error.is_none());
    });
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::CommandResult {
                correlation_id: request.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some("{\"selected_path\":null}".into()),
                },
            },
            cx,
        )
    });
    owner.read_with(cx, |owner, _| {
        assert!(owner.picker.is_none());
        assert!(owner.error.is_none());
    });
}

#[gpui_kit::test]
fn runtime_feedback_picker_disconnected_reply_cannot_change_new_selection(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.pick_image(cx));
    let old = next(&endpoint).id;
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![old],
            },
            cx,
        )
    });
    owner.update(cx, |owner, _| owner.error = None);
    owner.update(cx, |owner, cx| owner.pick_image(cx));
    let current = next(&endpoint).id;
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::CommandResult {
                correlation_id: old,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some("{\"selected_path\":\"C:\\\\synthetic-old.png\"}".into()),
                },
            },
            cx,
        )
    });
    owner.read_with(cx, |owner, _| {
        assert_eq!(owner.picker, Some(current));
        assert!(owner.settings.image_path.is_none());
        assert!(owner.error.is_none());
    });
}

fn fill_floating_queue(owner: &Entity<FloatingAssistantApplication>, cx: &mut TestAppContext) {
    owner.read_with(cx, |owner, _| {
        for _ in 0..64 {
            owner
                .platform
                .send_command(PlatformCommand::FloatingAssistant(
                    owner.command(FloatingAssistantOperation::Update, None),
                ))
                .expect("fill original bounded queue");
        }
    });
}

#[gpui_kit::test]
fn floating_drop_release_full_queue_recovers_after_cancel(cx: &mut TestAppContext) {
    for cancel in [false, true] {
        let (owner, endpoint, _directory) = fixture(cx);
        owner.update(cx, |owner, cx| owner.start(cx));
        let prepare = next(&endpoint);
        reply(&owner, prepare.id, true, cx);
        let show = next(&endpoint);
        reply(&owner, show.id, true, cx);
        let id = Uuid::new_v4();
        let mut event = drop_event(&owner, cx, None, 1, id);
        event.image_format = None;
        event.text = Some("完整 搜索 文本".into());
        fill_floating_queue(&owner, cx);
        owner.update(cx, |owner, cx| {
            if cancel {
                owner.dispatch_event(event.clone(), cx);
                owner.cancel_actions(false);
            } else {
                owner.settings.enabled = false;
                owner.native_event(event.clone(), cx);
            }
        });
        cx.run_until_parked();
        let mut releases = Vec::new();
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::FloatingAssistant(command)) =
                envelope.message
                && command.operation == FloatingAssistantOperation::ReleaseDrop
            {
                releases.push(command);
            }
        }
        assert_eq!(
            releases.len(),
            1,
            "full queue must retain exact release through cancellation"
        );
        assert_eq!(releases[0].session_id, event.session_id);
        assert_eq!(releases[0].generation, event.generation);
        assert_eq!(releases[0].drop_id, Some(id));
    }
}

#[gpui_kit::test]
fn floating_drop_retire_full_queue_survives_owner_drop(cx: &mut TestAppContext) {
    for destroy in [false, true] {
        let (owner, endpoint, _directory) = fixture(cx);
        owner.update(cx, |owner, cx| owner.start(cx));
        let prepare = next(&endpoint);
        reply(&owner, prepare.id, true, cx);
        let show = next(&endpoint);
        reply(&owner, show.id, true, cx);
        fill_floating_queue(&owner, cx);
        if destroy {
            let weak = owner.downgrade();
            drop(owner);
            cx.update(|_| {});
            cx.run_until_parked();
            assert!(
                weak.upgrade().is_none(),
                "fixture must actually destroy the controller"
            );
        } else {
            owner.update(cx, |owner, cx| owner.retry(cx));
        }
        let mut retires = 0;
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if operation(&envelope) == FloatingAssistantOperation::Retire {
                retires += 1;
            }
        }
        assert_eq!(
            retires, 1,
            "original connection must retain retirement without a live controller"
        );
    }
}

#[gpui_kit::test]
fn floating_drop_release_disconnect_and_stale_generation_never_replay(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let old_event = drop_event(&owner, cx, None, 1, Uuid::new_v4());
    fill_floating_queue(&owner, cx);
    owner.update(cx, |owner, _| owner.release_drop(old_event.drop_id));
    endpoint.set_connected_for_contract_test(false);
    endpoint.set_connected_for_contract_test(true);
    // Before the UI dispatcher observes EOF, the original connection still cannot
    // authorize an old controller's release or destructor on the replacement pipe.
    owner.update(cx, |owner, _| owner.release_drop(old_event.drop_id));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "old connection release cannot enter replacement queue"
    );
    owner.update(cx, |owner, cx| {
        owner.handle(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![],
            },
            cx,
        );
        owner.native_event(old_event, cx);
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn floating_drop_late_action_result_cannot_rebind_release_generation(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let id = Uuid::new_v4();
    let mut event = drop_event(&owner, cx, None, 1, id);
    event.image_format = None;
    event.text = Some("完整 搜索 文本".into());
    owner.update(cx, |owner, cx| {
        owner.dispatch_event(event, cx);
        owner.generation += 1;
        owner.complete_action(
            id,
            &PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "old completion must not release using the new generation"
    );
}

#[gpui_kit::test]
fn floating_drop_retire_recovers_after_original_deadlines(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture(cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    let generation = owner.read_with(cx, |owner, _| owner.generation);
    fill_floating_queue(&owner, cx);
    owner.update(cx, |owner, cx| owner.retry(cx));
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(60));
    cx.run_until_parked();
    assert!(
        owner.read_with(cx, |owner, _| owner
            .lifecycle
            .pending
            .is_some_and(|(_, phase)| phase == Phase::Retire)),
        "retained retirement cannot lose its owner to a generic timeout"
    );
    let pending = owner.read_with(cx, |owner, _| owner.lifecycle.pending);
    owner.update(cx, |owner, cx| owner.retry(cx));
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.lifecycle.pending),
        pending,
        "retry cannot replace the still-owned original retirement"
    );
    let mut retirement = None;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if operation(&envelope) == FloatingAssistantOperation::Retire {
            retirement = Some(envelope.id);
        }
    }
    reply(&owner, retirement.expect("late retained retire"), true, cx);
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.generation),
        generation + 1
    );
    assert_eq!(
        operation(&next(&endpoint)),
        FloatingAssistantOperation::Prepare
    );
}
