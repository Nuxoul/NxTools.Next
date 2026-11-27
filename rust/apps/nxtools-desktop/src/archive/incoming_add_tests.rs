use super::*;
use gpui_kit::{MouseButton, MouseDownEvent, MouseUpEvent, Role, VisualTestContext};
use nxtools_protocol::archive_job::{ArchiveJobAction, ArchiveJobModifyAction};

fn click_add(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    let position = surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(Role::Button) && node.label() == Some("添加到当前压缩包")
            })
            .expect("original Add button")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position,
        click_count: 1,
        ..Default::default()
    });
    surface.run_until_parked();
}

fn qualified_fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    Uuid,
    Uuid,
) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let context = slot.browser.as_mut().expect("context");
        context.format = "Tar".into();
        context.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let context = context.clone();
        slot.window
            .update(cx, |_, window, cx| {
                slot.view
                    .update(cx, |view, cx| {
                        let entries = view.model.as_ref().expect("model").entries.clone();
                        view.install(snapshot, entries, "Tar".into(), window, cx);
                        view.bind_edit_context(&context, cx);
                        assert!(view.edit_reason().is_none());
                    })
                    .expect("view");
            })
            .expect("window");
    });
    (owner, endpoint, id, snapshot)
}

#[gpui_kit::test]
fn incoming_add_button_submits_once_without_form(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = qualified_fixture(cx);
    let paths = vec![
        PathBuf::from(r"C:\owned\中文 文件.txt"),
        PathBuf::from(r"C:\owned\文件 目录"),
    ];
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let original = owner.read_with(cx, |app, _| {
        app.windows[&id].browser.clone().expect("browser")
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    owner.update(&mut surface, |app, cx| {
        app.handle_drop(id, paths.clone(), window, cx)
    });
    click_add(&mut surface);
    owner.read_with(&surface, |app, _| {
        assert!(
            app.jobs.forms.is_empty(),
            "explicit Add must not create an editor, including a hidden form"
        );
        assert_eq!(app.jobs.records.len(), 1, "one original queued job");
        let record = app.jobs.records.values().next().expect("job");
        assert_eq!(
            record.edit_origin,
            Some((id, app.windows[&id].generation, snapshot))
        );
        assert!(record.edit_form.is_none());
    });
    let mut jobs = Vec::new();
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = envelope.message {
            jobs.push(command);
        }
    }
    assert_eq!(jobs.len(), 1, "actual typed IPC once");
    assert_eq!(jobs[0].action, ArchiveJobAction::Start);
    let request = jobs[0].request.as_ref().expect("request");
    assert_eq!(
        request.paths,
        std::iter::once(r"C:\owned\preview.zip".to_owned())
            .chain(
                paths
                    .iter()
                    .map(|path| path.to_str().expect("Unicode").to_owned())
            )
            .collect::<Vec<_>>()
    );
    let modify = request.modify.as_ref().expect("Modify");
    assert_eq!(modify.action, ArchiveJobModifyAction::Add);
    assert!(
        modify.browser == original,
        "original context, not later current state"
    );
    assert!(modify.value.is_none(), "root target");
}

#[gpui_kit::test]
fn incoming_add_button_uses_requested_name(cx: &mut TestAppContext) {
    let (owner, _, id, _) = preview_fixture(cx);
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = VisualTestContext::from_window(window, cx);
    owner.update(&mut surface, |app, cx| {
        app.handle_drop(id, vec![r"C:\owned\file.txt".into()], window, cx)
    });
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert!(
            nodes
                .iter()
                .any(|node| node.label() == Some("添加到当前压缩包")),
            "actual rendered Button label"
        );
        assert!(
            !nodes
                .iter()
                .any(|node| node.label() == Some("添加到当前归档"))
        );
    });
}

#[gpui_kit::test]
fn incoming_add_failure_uses_operation_notice_once(cx: &mut TestAppContext) {
    use crate::notifications::{NotificationCenter, NotificationLevel};
    use nxtools_protocol::window_material::{
        AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason, MaterialSurface,
    };
    let (owner, endpoint, id, _) = preview_fixture(cx);
    let center = cx.new(|_| NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    let job = owner.update(cx, |app, cx| {
        let slot = &app.windows[&id];
        let request = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser: slot.browser.clone().expect("browser"),
            action: ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: None,
            source_prefix: None,
            added: vec![r"C:\owned\file.txt".into()],
            codepage: 0,
        }
        .request()
        .expect("request");
        let origin = (
            id,
            slot.generation,
            slot.browser.as_ref().expect("context").snapshot_id,
        );
        let job = app
            .submit_job(
                request,
                crate::archive::forms::FormCompletion::default(),
                cx,
            )
            .expect("job");
        app.jobs.records.get_mut(&job).expect("record").edit_origin = Some(origin);
        job
    });
    cx.run_until_parked();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let main: gpui_kit::AnyWindowHandle = cx
        .update(|cx| {
            cx.open_window(
                gpui_kit::WindowOptions {
                    show: false,
                    focus: false,
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| gpui_kit::base::Root::new(cx.new(|_| PickerHost), window, cx))
                },
            )
        })
        .expect("hidden main fixture")
        .into();
    let platform = owner.read_with(cx, |app, _| app.platform.clone());
    cx.update(|cx| cx.set_reduce_motion(true));
    cx.update(|cx| crate::notifications::screen::attach(center.clone(), platform, main, cx));
    let before = cx.read(|cx| cx.windows());
    let terminal: nxtools_protocol::archive_job::ArchiveJobTerminal = serde_json::from_value(serde_json::json!({
        "outcome":"failed", "code":"unsafe_path", "files_committed":0, "bytes_committed":0,
        "files_verified":0, "bytes_verified":0, "files_skipped":0, "cleanup_complete":true,
        "commit_state":null, "elapsed_ms":1, "native_error_code":null, "journal":[], "output_published":false
    })).expect("original native terminal");
    owner.update(cx, |app, cx| {
        app.job_terminal(job, terminal.clone(), cx);
        app.job_terminal(job, terminal, cx);
    });
    cx.run_until_parked();
    center.read_with(cx, |center, _| {
        assert_eq!(center.items().len(), 1, "one original terminal notice");
        let notice = &center.items()[0];
        assert_eq!(notice.title.as_ref(), "添加文件失败");
        assert_eq!(notice.level, NotificationLevel::Error);
        assert!(notice.message.contains("路径") && notice.message.contains("校验"));
        assert!(notice.message.contains("原压缩包未被替换"));
        assert!(!notice.message.contains("unsafe_path") && !notice.message.contains("可用空间"));
        assert_eq!(
            notice.target,
            Some(owner.read_with(cx, |app, _| app.windows[&id].window))
        );
    });
    let popup = cx.read(|cx| {
        let added = cx
            .windows()
            .into_iter()
            .filter(|window| !before.contains(window))
            .collect::<Vec<_>>();
        assert_eq!(
            added.len(),
            1,
            "one original independent notification surface"
        );
        added[0]
    });
    assert_ne!(popup, main);
    let mut popup_hwnd = None;
    let mut applied_material = false;
    let mut revealed = false;
    for _ in 0..8 {
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("notification native phase");
        let mut material = false;
        let data = match wire.message {
            Message::PlatformCommand(PlatformCommand::PrepareNotificationWindow(prepare)) => {
                assert!(!prepare.keep_visible, "hidden preparation before reveal");
                popup_hwnd = Some(prepare.window_handle);
                cx.update(|cx| {
                    crate::ui::material::runtime::set_test_window_handle(
                        popup,
                        prepare.window_handle,
                        cx,
                    )
                });
                let surface = VisualTestContext::from_window(popup, cx);
                surface.simulate_scale_factor_change(1.0);
                surface.simulate_resize(gpui_kit::size(
                    px(prepare.width as f32),
                    px(prepare.height as f32),
                ));
                Some(format!(
                    "{{\"width\":{},\"height\":{},\"dpi\":96}}",
                    prepare.width, prepare.height
                ))
            }
            Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) => {
                assert_eq!(Some(command.window_handle), popup_hwnd);
                assert_eq!(command.surface, MaterialSurface::Transparent);
                assert!(!applied_material && !revealed);
                let receipt = AppliedWindowMaterial {
                    session_id: command.session_id,
                    facts_revision: command.facts_revision,
                    window_handle: command.window_handle,
                    generation: command.generation,
                    style: command.style,
                    surface: command.surface,
                    dark: command.dark,
                    effective: EffectiveWindowMaterial::Transparent,
                    fallback_reason: MaterialFallbackReason::None,
                    native_error_code: 0,
                };
                material = true;
                Some(serde_json::to_string(&receipt).expect("typed transparent receipt"))
            }
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(show)) => {
                assert!(show.visible);
                assert!(applied_material, "transparent receipt precedes reveal");
                assert_eq!(Some(show.window_handle), popup_hwnd);
                assert!(!revealed, "no duplicate reveal");
                revealed = true;
                None
            }
            Message::PlatformCommand(PlatformCommand::SetTaskbarProgress(_)) => continue,
            Message::PlatformCommand(command) => {
                let kind = serde_json::to_value(&command).expect("command tag");
                panic!("unexpected native phase tag: {:?}", kind.get("command"));
            }
            _ => panic!("unexpected noncommand notification phase"),
        };
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data,
        };
        endpoint
            .route_command_result_for_contract_test(wire.id, result.clone())
            .expect("original notification router terminal");
        cx.update(|cx| {
            let notification = PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result,
            };
            if material {
                assert!(crate::ui::material::runtime::handle_notification(
                    &notification,
                    cx
                ));
                applied_material = true;
            } else {
                crate::notifications::screen::handle(notification, cx);
            }
        });
        cx.run_until_parked();
        if revealed {
            break;
        }
        let mut surface = VisualTestContext::from_window(popup, cx);
        surface.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
        });
        cx.run_until_parked();
    }
    assert!(
        revealed,
        "business failure reached original hidden/frame/reveal chain while main stays hidden"
    );
    assert_eq!(center.read_with(cx, |center, _| center.items().len()), 1);
}

#[gpui_kit::test]
#[ignore = "requires the owned native Add fixture"]
fn incoming_add_native_request_fixture(cx: &mut TestAppContext) {
    #[derive(serde::Deserialize)]
    struct Input {
        path: PathBuf,
        sources: Vec<PathBuf>,
        folder: String,
        browser: ArchiveBrowserContext,
        entries: Vec<ArchiveReadEntry>,
    }
    let root = PathBuf::from(
        std::env::var_os("NXTOOLS_INCOMING_ADD_UI_ROOT").expect("owned native fixture"),
    );
    let input: Input =
        serde_json::from_slice(&std::fs::read(root.join("ui-input.json")).expect("input"))
            .expect("typed native browse");
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(input.path.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let (id, window) = owner.update(cx, |app, cx| {
        app.reads.clear();
        let connection = app.platform.connection_generation().expect("connection");
        let platform = app.platform.clone();
        let (id, slot) = app.windows.iter_mut().next().expect("browser");
        slot.browser = Some(input.browser.clone());
        slot.window
            .update(cx, |_, window, cx| {
                slot.view
                    .update(cx, |view, cx| {
                        view.install(
                            input.browser.snapshot_id,
                            input.entries.clone(),
                            input.browser.format.clone(),
                            window,
                            cx,
                        );
                        view.configure_drag(platform, input.browser.clone(), connection, 0, false);
                        view.bind_edit_context(&input.browser, cx);
                        view.model.as_mut().expect("model").enter(&input.folder);
                    })
                    .expect("view");
            })
            .expect("window");
        (*id, slot.window)
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        app.windows[&id]
            .view
            .read_with(cx, |view, _| {
                assert!(view.edit_reason().is_none(), "actual source is qualified");
                assert_eq!(view.model.as_ref().expect("model").folder, input.folder);
            })
            .expect("view");
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let mut surface = VisualTestContext::from_window(window, cx);
    owner.update(&mut surface, |app, cx| {
        app.handle_drop(id, input.sources.clone(), window, cx)
    });
    click_add(&mut surface);
    owner.read_with(&surface, |app, _| {
        assert!(app.jobs.forms.is_empty());
        assert_eq!(app.jobs.records.len(), 1);
    });
    let mut commands = Vec::new();
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = wire.message {
            commands.push(command);
        }
    }
    assert_eq!(commands.len(), 1, "one actual Desktop request");
    let request = commands[0].request.as_ref().expect("request");
    assert_eq!(request.paths.len(), input.sources.len() + 1);
    assert_eq!(
        request
            .modify
            .as_ref()
            .expect("modify")
            .value
            .as_deref()
            .unwrap_or("")
            .trim_end_matches('/'),
        input.folder
    );
    assert!(request.modify.as_ref().expect("modify").browser == input.browser);
    std::fs::write(
        root.join("ui-command.json"),
        serde_json::to_vec(&commands[0]).expect("typed command"),
    )
    .expect("original command export");
    println!(
        "INCOMING_DESKTOP request_emitted=1 forms=0 sources={}",
        input.sources.len()
    );
}

fn stale_button(cx: &mut TestAppContext, change: &str) {
    let (owner, endpoint, id, _) = qualified_fixture(cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = VisualTestContext::from_window(window, cx);
    owner.update(&mut surface, |app, cx| {
        app._drain = cx.spawn(async move |_, _| {});
        app.handle_drop(id, vec![r"C:\owned\incoming.txt".into()], window, cx);
    });
    surface.run_until_parked();
    let replaced = (change == "connection").then(|| endpoint.open_connection_for_contract_test());
    owner.update(&mut surface, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("original");
        match change {
            "generation" => slot.generation += 1,
            "source" => slot.path = r"C:\owned\other.tar".into(),
            "snapshot" => slot.browser.as_mut().expect("context").snapshot_id = Uuid::new_v4(),
            "folder" => {
                slot.view
                    .update(cx, |view, _| {
                        view.model.as_mut().expect("model").folder = "changed".into()
                    })
                    .expect("view");
            }
            "tab" => {
                app.host
                    .as_ref()
                    .expect("host")
                    .view
                    .update(cx, |host, _| host.active = None)
                    .expect("host view");
            }
            "connection" => {}
            _ => panic!("fixed test case"),
        }
    });
    click_add(&mut surface);
    owner.read_with(&surface, |app, _| {
        assert!(
            app.jobs.records.is_empty() && app.jobs.forms.is_empty(),
            "stale original cannot submit or open form"
        )
    });
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            wire.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ));
    }
    if let Some(receiver) = replaced {
        while let Ok(wire) = receiver.try_recv() {
            assert!(!matches!(
                wire.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            ));
        }
    }
    center.read_with(&surface, |center, _| {
        assert_eq!(center.items().len(), 1);
        assert_eq!(center.items()[0].title.as_ref(), "添加文件失败");
    });
}

#[gpui_kit::test]
fn incoming_add_changed_folder_refuses_actual_button(cx: &mut TestAppContext) {
    stale_button(cx, "folder");
}
#[gpui_kit::test]
fn incoming_add_changed_snapshot_refuses_actual_button(cx: &mut TestAppContext) {
    stale_button(cx, "snapshot");
}
#[gpui_kit::test]
fn incoming_add_changed_generation_refuses_actual_button(cx: &mut TestAppContext) {
    stale_button(cx, "generation");
}
#[gpui_kit::test]
fn incoming_add_changed_source_refuses_actual_button(cx: &mut TestAppContext) {
    stale_button(cx, "source");
}
#[gpui_kit::test]
fn incoming_add_changed_tab_refuses_actual_button(cx: &mut TestAppContext) {
    stale_button(cx, "tab");
}
#[gpui_kit::test]
fn incoming_add_replaced_connection_refuses_before_disconnect_delivery(cx: &mut TestAppContext) {
    stale_button(cx, "connection");
}

#[gpui_kit::test]
fn incoming_add_closed_origin_cannot_revive(cx: &mut TestAppContext) {
    let (owner, endpoint, id, _) = qualified_fixture(cx);
    owner.update(cx, |app, cx| {
        let target = app
            .drop_add_target(id, &[r"C:\owned\incoming.txt".into()], cx)
            .expect("frozen");
        app.close_browser(id, cx);
        app.add_drop(target, cx);
        assert!(app.jobs.records.is_empty() && app.jobs.forms.is_empty());
    });
    cx.run_until_parked();
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            wire.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ));
    }
}

fn failed_terminal() -> nxtools_protocol::archive_job::ArchiveJobTerminal {
    serde_json::from_value(serde_json::json!({
        "outcome":"failed", "code":"unsafe_path", "files_committed":0, "bytes_committed":0,
        "files_verified":0, "bytes_verified":0, "files_skipped":0, "cleanup_complete":true,
        "commit_state":null, "elapsed_ms":1, "native_error_code":null, "journal":[], "output_published":false
    })).expect("native unpublished failure")
}

#[gpui_kit::test]
fn incoming_add_queued_target_change_never_sends(cx: &mut TestAppContext) {
    let (owner, endpoint, id, _) = qualified_fixture(cx);
    let (blocking, queued) = owner.update(cx, |app, cx| {
        let target = app
            .drop_add_target(id, &[r"C:\owned\incoming.txt".into()], cx)
            .expect("frozen");
        let blocking = app
            .submit_job(
                target.request.clone(),
                crate::archive::forms::FormCompletion::default(),
                cx,
            )
            .expect("blocking job");
        app.add_drop(target, cx);
        assert_eq!(app.jobs.records.len(), 2);
        let queued = *app
            .jobs
            .records
            .keys()
            .find(|job| **job != blocking)
            .expect("queued Add");
        assert_eq!(app.jobs.records[&queued].state, "排队中");
        (blocking, queued)
    });
    cx.run_until_parked();
    let mut starts = 0;
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if matches!(
            wire.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ) {
            starts += 1;
        }
    }
    assert_eq!(starts, 1, "only blocker started");
    owner.update(cx, |app, cx| {
        app.windows[&id]
            .view
            .update(cx, |view, _| {
                view.model.as_mut().expect("model").folder = "changed".into()
            })
            .expect("view");
        app.job_terminal(blocking, failed_terminal(), cx);
        app.pump_jobs(cx);
        assert_eq!(
            app.jobs.records[&queued]
                .terminal
                .as_ref()
                .expect("local refusal")
                .code
                .as_deref(),
            Some("stale_add_target")
        );
    });
    cx.run_until_parked();
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                wire.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            ),
            "no stale queued native Start"
        );
    }
}

#[gpui_kit::test]
fn incoming_add_form_terminal_uses_one_notice_and_retains_sources(cx: &mut TestAppContext) {
    let (owner, _, id, snapshot) = qualified_fixture(cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    let target = owner.update(cx, |app, cx| {
        let slot = &app.windows[&id];
        let target = app
            .drop_add_target(id, &[r"C:\owned\incoming.txt".into()], cx)
            .expect("frozen request");
        let draft = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser: slot.browser.clone().expect("context"),
            action: ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: None,
            source_prefix: None,
            added: vec![r"C:\owned\incoming.txt".into()],
            codepage: 0,
        };
        app.open_modify_form(draft, target.origin.browser, cx);
        target
    });
    cx.run_until_parked();
    let (form, generation, window, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.jobs.forms.iter().next().expect("deliberate Add form");
        (*id, slot.generation, slot.window, slot.view.clone())
    });
    owner.update(cx, |app, cx| {
        app.submit_form(
            form,
            generation,
            target.request,
            crate::archive::forms::FormCompletion::default(),
            cx,
        );
        let job = *app
            .jobs
            .records
            .keys()
            .next()
            .expect("original submitted job");
        assert_eq!(
            app.jobs.records[&job].edit_origin,
            Some((id, app.windows[&id].generation, snapshot))
        );
        app.job_terminal(job, failed_terminal(), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.sources_for_test().1)
            .expect("retained form"),
        vec![PathBuf::from(r"C:\owned\incoming.txt")]
    );
    center.read_with(cx, |center, _| {
        assert_eq!(center.items().len(), 1);
        assert_eq!(center.items()[0].title.as_ref(), "添加文件失败");
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(
            !gpui_kit::base::test_support::snapshots(window)
                .iter()
                .filter_map(|node| node.label())
                .any(|text| text.contains("编辑未完成") || text.contains("unsafe_path"))
        );
    });
}
