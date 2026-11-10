use super::*;
use nxtools_protocol::archive_clipboard::{ArchiveClipboardCommand, ArchiveClipboardResult};

fn paste_request(
    surface: &mut gpui_kit::VisualTestContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> ArchiveClipboardCommand {
    object_draw(surface);
    object_click(surface, "粘贴文件");
    surface.run_until_parked();
    let mut request = None;
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ReadClipboardFileDrop(command)) =
            wire.message
        {
            assert_eq!(wire.id, command.operation_id);
            assert_eq!(wire.correlation_id, Some(command.operation_id));
            assert!(
                request.replace(command).is_none(),
                "one native request per click"
            );
        }
    }
    request.expect("production clipboard command")
}
fn paste_reply(
    owner: &Entity<ArchiveApplication>,
    command: &ArchiveClipboardCommand,
    paths: Vec<String>,
    cx: &mut gpui_kit::TestAppContext,
) {
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: command.operation_id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(
                        serde_json::to_string(&ArchiveClipboardResult {
                            version: 1,
                            operation_id: command.operation_id,
                            session_id: command.session_id,
                            generation: command.generation,
                            paths,
                            code: None,
                            native_error_code: None,
                        })
                        .expect("serialized native result"),
                    ),
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
}
#[gpui_kit::test]
fn opt03_paste_full_batch_empty_overflow_timeout_close_and_late(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
    cx.run_until_parked();
    let (id, window, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.jobs.forms.iter().next().expect("form");
        (*id, slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let root = std::env::temp_dir().join(format!("archive-opt03-paste-{}", Uuid::new_v4()));
    std::fs::create_dir_all(root.join("directory")).expect("owned root");
    std::fs::write(root.join("input.txt"), b"unchanged").expect("owned input");
    let paths = vec![root.join("input.txt"), root.join("directory")];
    let command = paste_request(&mut surface, &endpoint);
    assert!(
        owner.read_with(&surface, |app, _| app
            .jobs
            .clipboards
            .contains_key(&command.operation_id)),
        "pending before command is observable"
    );
    assert!(
        view.read_with(&surface, |form, _| form.busy())
            .expect("busy form")
    );
    paste_reply(
        &owner,
        &command,
        paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        cx,
    );
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test())
            .expect("form"),
        (false, paths.clone())
    );
    for incoming in [
        vec![],
        vec![r"C:\too-many.txt".into(); 33],
        vec![r"C:\valid.txt".into(), String::new()],
    ] {
        let command = paste_request(&mut surface, &endpoint);
        paste_reply(&owner, &command, incoming, cx);
        assert_eq!(
            view.read_with(cx, |form, _| form.sources_for_test().1)
                .expect("form"),
            paths
        );
        assert!(
            !view
                .read_with(cx, |form, _| form.busy())
                .expect("released form")
        );
    }
    let timeout = paste_request(&mut surface, &endpoint);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(8));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.jobs.clipboards.is_empty()));
    assert!(
        !view
            .read_with(cx, |form, _| form.busy())
            .expect("timeout releases form")
    );
    paste_reply(&owner, &timeout, vec![r"C:\late.txt".into()], cx);
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("form"),
        paths
    );
    let closing = paste_request(&mut surface, &endpoint);
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    assert!(
        owner.read_with(cx, |app, _| !app.jobs.forms.contains_key(&id)
            && app.jobs.clipboards.is_empty())
    );
    paste_reply(&owner, &closing, vec![r"C:\late-closed.txt".into()], cx);
    assert!(owner.read_with(cx, |app, _| app.jobs.forms.is_empty()
        && app.jobs.records.is_empty()));
    assert_eq!(
        std::fs::read(root.join("input.txt")).expect("source retained"),
        b"unchanged"
    );
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}
#[gpui_kit::test]
fn opt03_paste_add_freezes_folder_snapshot_and_disconnect(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint, id, snapshot) = super::super::super::tests::preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "Tar".into();
        browser.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = browser.clone();
        let generation = slot.generation;
        let view = slot.view.upgrade().expect("view");
        slot.window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("model").entries.clone();
                    view.install(snapshot, entries, "Tar".into(), window, cx);
                    view.bind_edit_context(&browser, cx);
                    view.model.as_mut().expect("model").folder.clear();
                })
            })
            .expect("window");
        let draft = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser,
            action: ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: None,
            source_prefix: None,
            added: vec![r"C:\initial.txt".into()],
            codepage: 0,
        };
        app.open_modify_form(draft, (id, generation, snapshot), cx);
    });
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("Add form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let command = paste_request(&mut surface, &endpoint);
    paste_reply(&owner, &command, vec![r"C:\accepted.txt".into()], cx);
    let accepted = vec![
        PathBuf::from(r"C:\initial.txt"),
        PathBuf::from(r"C:\accepted.txt"),
    ];
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("form"),
        accepted
    );
    for change in ["folder", "snapshot", "disconnect"] {
        let command = paste_request(&mut surface, &endpoint);
        owner.update(cx, |app, cx| {
            if change == "disconnect" {
                let generation = app.platform.connection_generation().expect("connection");
                app.notification(
                    PlatformNotification::Disconnected {
                        generation,
                        pending_command_ids: vec![command.operation_id],
                    },
                    cx,
                );
            } else {
                let slot = app.windows.get(&id).expect("browser");
                slot.view
                    .update(cx, |view, _| {
                        let model = view.model.as_mut().expect("model");
                        if change == "folder" {
                            model.folder = "elsewhere".into();
                        } else {
                            model.snapshot = Uuid::new_v4();
                        }
                    })
                    .expect("view");
            }
        });
        paste_reply(&owner, &command, vec![r"C:\must-not-append.txt".into()], cx);
        assert_eq!(
            view.read_with(cx, |form, _| form.sources_for_test().1)
                .expect("form"),
            accepted
        );
        assert!(
            !view
                .read_with(cx, |form, _| form.busy())
                .expect("released form")
        );
        owner.update(cx, |app, cx| {
            app.windows[&id]
                .view
                .update(cx, |view, _| {
                    let model = view.model.as_mut().expect("model");
                    model.folder.clear();
                    model.snapshot = snapshot;
                })
                .expect("reset fixture view");
        });
    }
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()));
}

#[gpui_kit::test]
fn opt03_paste_receipt_then_disconnect_before_deferred_apply(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let command = paste_request(&mut surface, &endpoint);
    let root = std::env::temp_dir().join(format!("archive-opt03-retired-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("owned root");
    let input = root.join("input.txt");
    std::fs::write(&input, b"retained").expect("owned source");
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: command.operation_id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(
                        serde_json::to_string(&ArchiveClipboardResult {
                            version: 1,
                            operation_id: command.operation_id,
                            session_id: command.session_id,
                            generation: command.generation,
                            paths: vec![input.to_string_lossy().into_owned()],
                            code: None,
                            native_error_code: None,
                        })
                        .expect("native wire"),
                    ),
                },
            },
            cx,
        );
        endpoint.set_connected_for_contract_test(false);
    });
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test())
            .expect("form"),
        (false, vec![]),
        "retired connection cannot append after receipt"
    );
    assert!(
        !view
            .read_with(cx, |form, _| form.busy())
            .expect("pending released")
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.clipboards.is_empty()));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[gpui_kit::test]
fn opt03_add_picker_rechecks_frozen_folder(cx: &mut gpui_kit::TestAppContext) {
    add_folder_boundary(cx, true);
}
#[gpui_kit::test]
fn opt03_add_submit_rechecks_frozen_folder(cx: &mut gpui_kit::TestAppContext) {
    add_folder_boundary(cx, false);
}
fn add_folder_boundary(cx: &mut gpui_kit::TestAppContext, picking: bool) {
    let (owner, endpoint, id, snapshot) = super::super::super::tests::preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "Tar".into();
        browser.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = browser.clone();
        let generation = slot.generation;
        let view = slot.view.upgrade().expect("view");
        slot.window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("model").entries.clone();
                    view.install(snapshot, entries, "Tar".into(), window, cx);
                    view.bind_edit_context(&browser, cx);
                    view.model.as_mut().expect("model").folder.clear();
                })
            })
            .expect("window");
        let draft = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser,
            action: ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: None,
            source_prefix: None,
            added: vec![r"C:\initial.txt".into()],
            codepage: 0,
        };
        app.open_modify_form(draft, (id, generation, snapshot), cx);
    });
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("Add form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    let mut picker = None;
    if picking {
        object_click(&mut surface, "添加文件…");
        surface.run_until_parked();
        while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::ArchivePicker(command)) = wire.message
            {
                picker = Some(command);
            }
        }
    }
    owner.update(cx, |app, cx| {
        app.windows[&id]
            .view
            .update(cx, |view, _| {
                view.model.as_mut().expect("model").folder = "elsewhere".into();
            })
            .expect("navigation");
    });
    if picking {
        let command = picker.expect("original picker dispatch");
        owner.update(cx, |app, cx| {
            app.notification(
                PlatformNotification::Event(PlatformEvent::ArchivePicker(ArchivePickerResult {
                    version: 1,
                    operation_id: command.operation_id,
                    session_id: command.session_id,
                    generation: command.generation,
                    cancelled: false,
                    paths: vec![r"C:\must-not-append.txt".into()],
                    code: None,
                    native_error_code: None,
                })),
                cx,
            )
        });
    } else {
        object_draw(&mut surface);
        object_click(&mut surface, "应用编辑");
    }
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("form"),
        vec![PathBuf::from(r"C:\initial.txt")],
        "picker cannot append after navigation"
    );
    assert!(
        !view
            .read_with(cx, |form, _| form.busy())
            .expect("released form")
    );
    assert!(
        owner.read_with(cx, |app, _| app.jobs.records.is_empty()),
        "final Add submission must reject a changed internal folder"
    );
}

fn add_draft_connection_case(
    cx: &mut gpui_kit::TestAppContext,
    action: &str,
    replace_connection: bool,
) {
    let (owner, endpoint, id, snapshot) = super::super::super::tests::preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "Tar".into();
        browser.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = browser.clone();
        slot.window
            .update(cx, |_, window, cx| {
                slot.view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("model").entries.clone();
                    view.install(snapshot, entries, "Tar".into(), window, cx);
                    view.bind_edit_context(&browser, cx);
                    view.model.as_mut().expect("model").folder.clear();
                })
            })
            .expect("browser window")
            .expect("browser view");
    });
    owner.update(cx, |app, cx| {
        let slot = &app.windows[&id];
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
        app.open_modify_form(draft, (id, slot.generation, snapshot), cx);
    });
    cx.run_until_parked();
    let (window, view, platform, original_browser) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("original Add form");
        (
            slot.window,
            slot.view.clone(),
            app.platform.clone(),
            app.windows[&id].browser.clone(),
        )
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let original_connection = platform
        .connection_generation()
        .expect("original connection");
    let notifications = platform.notification_receiver();
    assert!(notifications.is_empty());
    let replacement = replace_connection.then(|| endpoint.open_connection_for_contract_test());
    if replace_connection {
        assert_ne!(platform.connection_generation(), Some(original_connection));
        assert_eq!(
            notifications.len(),
            1,
            "old disconnect remains queued before the action"
        );
    }
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    object_click(&mut surface, action);
    surface.run_until_parked();
    let mut commands = Vec::new();
    loop {
        let wire = match &replacement {
            Some(receiver) => receiver.try_recv().map_err(|error| error.to_string()),
            None => endpoint.try_receive_command_for_contract_test(),
        };
        let Ok(wire) = wire else {
            break;
        };
        if let Message::PlatformCommand(
            command @ (PlatformCommand::ArchiveJob(_)
            | PlatformCommand::ArchivePicker(_)
            | PlatformCommand::ReadClipboardFileDrop(_)),
        ) = wire.message
        {
            commands.push(command);
        }
    }
    if replace_connection {
        assert!(
            commands.is_empty(),
            "stale Add draft must not dispatch {action} on a replacement connection"
        );
        assert_eq!(
            notifications.len(),
            1,
            "refusal does not depend on applying the old disconnect"
        );
        owner.read_with(cx, |app, _| {
            assert!(app.jobs.records.is_empty(), "no stale logical Add job");
            assert!(app.jobs.pickers.is_empty() && app.jobs.clipboards.is_empty());
            assert_eq!(
                app.windows[&id].browser, original_browser,
                "unchanged native/source identity"
            );
        });
        assert!(
            !view
                .read_with(cx, |form, _| form.busy())
                .expect("stale action settles")
        );
        assert_eq!(
            view.read_with(cx, |form, _| form.sources_for_test().1)
                .expect("retained draft"),
            vec![PathBuf::from(r"C:\owned\incoming.txt")]
        );
    } else {
        assert_eq!(
            commands.len(),
            1,
            "same-connection {action} retains original dispatch"
        );
        match (action, &commands[0]) {
            ("应用编辑", PlatformCommand::ArchiveJob(command)) => {
                assert_eq!(command.action, ArchiveJobAction::Start);
                assert_eq!(owner.read_with(cx, |app, _| app.jobs.records.len()), 1);
            }
            ("粘贴文件", PlatformCommand::ReadClipboardFileDrop(_))
            | ("添加文件…" | "添加文件夹…", PlatformCommand::ArchivePicker(_)) => {}
            _ => panic!("wrong original dispatch for {action}"),
        }
    }
}

#[gpui_kit::test]
fn opt03_ac01_add_submit_rejects_replacement_before_disconnect_delivery(
    cx: &mut gpui_kit::TestAppContext,
) {
    add_draft_connection_case(cx, "应用编辑", true);
}

#[gpui_kit::test]
fn opt03_ac01_add_intake_rejects_replacement_before_disconnect_delivery(
    cx: &mut gpui_kit::TestAppContext,
) {
    for action in ["粘贴文件", "添加文件…", "添加文件夹…"] {
        add_draft_connection_case(cx, action, true);
    }
}

#[gpui_kit::test]
fn opt03_ac01_add_same_connection_keeps_original_actions(cx: &mut gpui_kit::TestAppContext) {
    for action in ["应用编辑", "粘贴文件", "添加文件…", "添加文件夹…"] {
        add_draft_connection_case(cx, action, false);
    }
}

fn folder_request(
    surface: &mut gpui_kit::VisualTestContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> ArchivePickerCommand {
    object_draw(surface);
    object_click(surface, "添加文件夹…");
    surface.run_until_parked();
    let mut picked = None;
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchivePicker(command)) = wire.message {
            assert_eq!(wire.id, command.operation_id);
            assert_eq!(wire.correlation_id, Some(command.operation_id));
            assert_eq!(command.purpose, ArchivePickerPurpose::Folders);
            assert!(
                picked.replace(command).is_none(),
                "one owned folder request"
            );
        }
    }
    picked.expect("original folder picker command")
}

fn folder_reply(
    owner: &Entity<ArchiveApplication>,
    command: &ArchivePickerCommand,
    paths: Vec<String>,
    cx: &mut gpui_kit::TestAppContext,
) {
    let result = ArchivePickerResult {
        version: 1,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        paths,
        cancelled: false,
        code: None,
        native_error_code: None,
    };
    result.validate().expect("valid native folder vector");
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Event(PlatformEvent::ArchivePicker(result)),
            cx,
        )
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn opt07_folder_picker_rejects_combined_overlap_without_partial_draft(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("original New form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let root = std::env::temp_dir().join(format!("archive-opt07-folders-{}", Uuid::new_v4()));
    for relative in ["a/child", "ab", "unrelated"] {
        std::fs::create_dir_all(root.join(relative)).expect("owned folder");
    }
    let sentinel = root.join("a/child/source.txt");
    std::fs::write(&sentinel, b"source retained").expect("owned source");
    let original = vec![root.join("a"), root.join("ab")];
    let command = folder_request(&mut surface, &endpoint);
    assert!(
        owner.read_with(&surface, |app, _| app
            .jobs
            .pickers
            .contains_key(&command.operation_id)),
        "pending before original dispatch is observed"
    );
    folder_reply(
        &owner,
        &command,
        original
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        cx,
    );
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("form"),
        original,
        "component-adjacent folder names are independent"
    );
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(
        &owner,
        &command,
        [root.join("a").join("child"), root.join("unrelated")]
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        cx,
    );
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("form"),
        original,
        "overlap with an existing root must reject the entire incoming vector"
    );
    assert!(
        !view
            .read_with(cx, |form, _| form.busy())
            .expect("pending retired")
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()
        && app.jobs.pickers.is_empty()));
    assert_eq!(
        std::fs::read(sentinel).expect("source unchanged"),
        b"source retained"
    );
    std::fs::remove_dir_all(root).expect("owned fixture cleanup");
}

#[gpui_kit::test]
fn opt07_new_folder_vectors_keep_limits_cancel_failure_and_remove_readd(
    cx: &mut gpui_kit::TestAppContext,
) {
    for count in [2, 16, 32] {
        let (owner, endpoint) = super::super::super::tests::setup(cx);
        owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
        cx.run_until_parked();
        let (window, view) = owner.read_with(cx, |app, _| {
            let slot = app.jobs.forms.values().next().expect("New form");
            (slot.window, slot.view.clone())
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
        let root = std::env::temp_dir().join(format!("archive-opt07-roots-{}", Uuid::new_v4()));
        let paths = (0..count)
            .map(|i| root.join(format!("root-{i:02}")))
            .collect::<Vec<_>>();
        for path in &paths {
            std::fs::create_dir_all(path).expect("owned root");
        }
        let command = folder_request(&mut surface, &endpoint);
        folder_reply(
            &owner,
            &command,
            paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            cx,
        );
        assert_eq!(
            view.read_with(cx, |form, _| form.sources_for_test().1)
                .expect("form"),
            paths
        );
        if count == 2 {
            object_draw(&mut surface);
            object_click(&mut surface, "移除");
            surface.run_until_parked();
            let remaining = view
                .read_with(cx, |form, _| form.sources_for_test().1)
                .expect("removed");
            assert_eq!(remaining.len(), 1);
            let removed = paths
                .iter()
                .find(|path| !remaining.contains(path))
                .expect("exact one root removed");
            let command = folder_request(&mut surface, &endpoint);
            folder_reply(
                &owner,
                &command,
                vec![removed.to_string_lossy().into_owned()],
                cx,
            );
            assert_eq!(
                view.read_with(cx, |form, _| form.sources_for_test().1)
                    .expect("readded")
                    .len(),
                2
            );
        }
        let before = view
            .read_with(cx, |form, _| form.sources_for_test().1)
            .expect("retained");
        for cancelled in [true, false] {
            let command = folder_request(&mut surface, &endpoint);
            let result = ArchivePickerResult {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                paths: vec![],
                cancelled,
                code: (!cancelled).then(|| "picker_failed".into()),
                native_error_code: (!cancelled).then_some(5),
            };
            result.validate().expect("native terminal");
            owner.update(cx, |app, cx| {
                app.notification(
                    PlatformNotification::Event(PlatformEvent::ArchivePicker(result)),
                    cx,
                )
            });
            cx.run_until_parked();
            assert_eq!(
                view.read_with(cx, |form, _| form.sources_for_test().1)
                    .expect("unchanged"),
                before
            );
            assert!(!view.read_with(cx, |form, _| form.busy()).expect("settled"));
        }
        let command = folder_request(&mut surface, &endpoint);
        folder_reply(
            &owner,
            &command,
            vec![paths[0].to_string_lossy().into_owned()],
            cx,
        );
        assert_eq!(
            view.read_with(cx, |form, _| form.sources_for_test().1)
                .expect("atomic refusal"),
            before
        );
        assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()
            && app.jobs.pickers.is_empty()));
        std::fs::remove_dir_all(root).expect("owned cleanup");
    }
}

#[gpui_kit::test]
fn opt07_add_folder_vectors_preserve_combined_overlap_and_31_root_budget(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, id, snapshot) = super::super::super::tests::preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "Tar".into();
        browser.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = browser.clone();
        let generation = slot.generation;
        slot.window
            .update(cx, |_, window, cx| {
                slot.view.update(cx, |v, cx| {
                    let entries = v.model.as_ref().expect("catalog").entries.clone();
                    v.install(snapshot, entries, "Tar".into(), window, cx);
                    v.bind_edit_context(&browser, cx);
                    v.model.as_mut().expect("model").folder.clear();
                })
            })
            .expect("window")
            .expect("view");
        let draft = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser,
            action: ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: None,
            source_prefix: None,
            added: vec![r"C:\initial.txt".into()],
            codepage: 0,
        };
        app.open_modify_form(draft, (id, generation, snapshot), cx);
    });
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("Add form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(
        &owner,
        &command,
        vec![r"C:\owned\root".into(), r"C:\owned\root-adjacent".into()],
        cx,
    );
    let before = view
        .read_with(cx, |form, _| form.sources_for_test().1)
        .expect("original roots");
    assert_eq!(before.len(), 3);
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(
        &owner,
        &command,
        vec![r"C:\owned\root\child".into(), r"C:\owned\unrelated".into()],
        cx,
    );
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("atomic overlap refusal"),
        before
    );
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(
        &owner,
        &command,
        (0..14)
            .map(|i| format!(r"C:\owned\long-a-{i}-{}", "界".repeat(1600)))
            .collect(),
        cx,
    );
    let budget_before = view
        .read_with(cx, |form, _| form.sources_for_test().1)
        .expect("first legal vector");
    assert_eq!(budget_before.len(), 17);
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(
        &owner,
        &command,
        (0..14)
            .map(|i| format!(r"C:\owned\long-b-{i}-{}", "界".repeat(1600)))
            .collect(),
        cx,
    );
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("combined UTF8 refusal"),
        budget_before
    );
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(
        &owner,
        &command,
        (0..14).map(|i| format!(r"C:\owned\more-{i:02}")).collect(),
        cx,
    );
    let full = view
        .read_with(cx, |form, _| form.sources_for_test().1)
        .expect("31 roots");
    assert_eq!(full.len(), 31);
    let command = folder_request(&mut surface, &endpoint);
    folder_reply(&owner, &command, vec![r"C:\owned\overflow".into()], cx);
    assert_eq!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("root budget refusal"),
        full
    );
    assert!(!view.read_with(cx, |form, _| form.busy()).expect("settled"));
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()
        && app.jobs.pickers.is_empty()));
}

#[gpui_kit::test]
fn opt07_missing_native_multifolder_capability_refuses_without_pending_or_dispatch(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    endpoint.set_archive_multi_folder_picker_available_for_contract_test(false);
    owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("New form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    object_click(&mut surface, "添加文件夹…");
    surface.run_until_parked();
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |wire| matches!(
                wire.message,
                Message::PlatformCommand(PlatformCommand::ArchivePicker(_))
            )
        )
    );
    assert!(owner.read_with(&surface, |app, _| app.jobs.pickers.is_empty()));
    assert!(
        !view
            .read_with(&surface, |form, _| form.busy())
            .expect("settled")
    );
    assert!(
        view.read_with(&surface, |form, _| form.sources_for_test().1)
            .expect("unchanged draft")
            .is_empty()
    );
}

#[gpui_kit::test]
fn opt07_create_folder_vectors_enforce_combined_utf8_budget_before_mutation(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("New form");
        (slot.window, slot.view.clone())
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let command = folder_request(&mut surface, &endpoint);
    let paths = (0..32)
        .map(|i| format!(r"C:\owned\root-{i}-{}", "界".repeat(1500)))
        .collect::<Vec<_>>();
    let result = ArchivePickerResult {
        version: 1,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        paths,
        cancelled: false,
        code: None,
        native_error_code: None,
    };
    assert!(
        result.validate().is_err(),
        "aggregate UTF8 budget, not codepoint count"
    );
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Event(PlatformEvent::ArchivePicker(result)),
            cx,
        )
    });
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |form, _| form.sources_for_test().1)
            .expect("draft")
            .is_empty()
    );
    assert!(
        !view
            .read_with(cx, |form, _| form.busy())
            .expect("pending retired")
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.pickers.is_empty()
        && app.jobs.records.is_empty()));
}
