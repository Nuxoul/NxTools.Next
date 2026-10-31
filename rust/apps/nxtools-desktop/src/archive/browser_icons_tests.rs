//! Original ArchiveBrowser queue/result boundary; only synthetic metadata and owned PNGs.
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext, size};
use nxtools_protocol::{Message, PlatformCommand};
use std::cell::RefCell;

fn row(name: &str, directory: bool) -> Row {
    Row {
        id: RowId::Folder(Uuid::nil(), name.into()),
        name: name.into(),
        path: format!("untrusted/member/{name}"),
        directory,
        parent: false,
        size: None,
        packed_size: None,
        ordinal: None,
        encrypted: None,
        warning: false,
        kind: if directory { "Directory" } else { "File" }.into(),
        modified: None,
        method: None,
        crc32: None,
        attributes: None,
    }
}

#[test]
fn directories_share_one_safe_key_without_treating_navigation_or_warnings_as_types() {
    for name in ["folder", "looks-like.lnk", "looks-like.exe", "nested/other"] {
        assert_eq!(
            key(&row(name, true)).as_deref(),
            Some("nxtools-folder-type:directory")
        );
    }
    let mut parent = row("..", true);
    parent.parent = true;
    assert!(key(&parent).is_none());
    let mut warning = row("unsafe", true);
    warning.warning = true;
    assert!(key(&warning).is_none());
    assert_eq!(key(&row("FILE.EXE", false)).as_deref(), Some("exe"));
    assert_eq!(key(&row("shortcut.lnk", false)).as_deref(), Some("lnk"));
    assert!(key(&row("noextension", false)).is_none());
    assert!(key(&row("unsafe.bad-ext", false)).is_none());
}

fn mount(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveBrowser>,
    VisualTestContext,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = ArchiveBrowser::new(
                PathBuf::from(r"C:\synthetic\archive.zip"),
                Uuid::new_v4(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            view.install(Uuid::new_v4(), vec![], "Zip".into(), window, cx);
            view
        });
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(960.), px(640.)));
    draw(&mut surface);
    let view = retained.borrow().clone().expect("original browser");
    surface.update(|_, cx| view.update(cx, |view, _| view.configure_icons(Rc::new(bridge))));
    (view, surface, endpoint)
}

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
}

fn result(path: Option<&std::path::Path>) -> PlatformCommandResult {
    PlatformCommandResult {
        success: path.is_some(),
        error: None,
        native_error_code: None,
        data: path.map(|path| path.to_string_lossy().into_owned()),
    }
}

#[gpui_kit::test]
fn folder_rows_queue_one_type_and_share_validated_png_without_member_work(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    let rows = vec![row("first.lnk", true), row("second.exe", true)];
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.queue_type_icons(&rows, cx);
            view.queue_folder_icon(cx);
            assert!(
                endpoint.try_receive_command_for_contract_test().is_err(),
                "render only defers IPC"
            );
        })
    });
    draw(&mut surface);
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("directory type request");
    assert!(
        matches!(&command.message, Message::PlatformCommand(PlatformCommand::ExtractShellIcon(value)) if value.target == "nxtools-folder-type:directory" && value.size == 32)
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "one shared folder resource, never materialize/open"
    );
    let directory = tempfile::tempdir().expect("owned image");
    let image = directory.path().join("folder.png");
    std::fs::write(&image, crate::icons::ONE_PIXEL_PNG).expect("PNG");
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.icon_result(command.id, &result(Some(&image)), cx)
        })
    });
    draw(&mut surface);
    draw(&mut surface);
    view.read_with(&surface, |view, _| {
        for row in &rows {
            assert_eq!(view.icon_path(row), Some(&image));
        }
        assert_eq!(view.folder_icon_path(), Some(&image));
        assert!(!view.icon_matches(command.id));
    });
    surface.update(|_, cx| view.update(cx, |view, cx| view.queue_type_icons(&rows, cx)));
    draw(&mut surface);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "cache hit does not requeue or reenter owner"
    );
}

#[gpui_kit::test]
fn directory_results_obey_generation_snapshot_epoch_close_and_disconnect(cx: &mut TestAppContext) {
    for boundary in ["generation", "snapshot", "epoch", "close", "disconnect"] {
        let (view, mut surface, endpoint) = mount(cx);
        let folder = row("folder", true);
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.queue_type_icons(std::slice::from_ref(&folder), cx)
            })
        });
        draw(&mut surface);
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("folder request");
        let directory = tempfile::tempdir().expect("owned image");
        let image = directory.path().join("stale.png");
        std::fs::write(&image, crate::icons::ONE_PIXEL_PNG).expect("PNG");
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.icon_result(command.id, &result(Some(&image)), cx);
                match boundary {
                    "generation" => view.generation += 1,
                    "snapshot" => view.model.as_mut().expect("model").snapshot = Uuid::new_v4(),
                    "epoch" => view.clear_pending_icons(),
                    "close" => view.retire(cx),
                    "disconnect" => {
                        endpoint.set_connected_for_contract_test(false);
                        view.icons_disconnected();
                    }
                    _ => unreachable!(),
                }
            })
        });
        draw(&mut surface);
        draw(&mut surface);
        view.read_with(&surface, |view, _| {
            assert!(
                view.icon_path(&folder).is_none(),
                "stale {boundary} cannot publish PNG"
            );
            assert!(!view.icon_matches(command.id));
        });
    }
}

#[gpui_kit::test]
fn folder_failure_is_bounded_and_retries_only_after_original_epoch_reset(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    let folder = row("folder", true);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.queue_type_icons(std::slice::from_ref(&folder), cx)
        })
    });
    draw(&mut surface);
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("folder request");
    view.read_with(&surface, |view, _| {
        assert!(view.folder_icon_description().contains("正在加载"))
    });
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.icon_result(command.id, &result(None), cx)
        })
    });
    draw(&mut surface);
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.queue_type_icons(std::slice::from_ref(&folder), cx)
        })
    });
    draw(&mut surface);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "failed type never spins per redraw"
    );
    view.read_with(&surface, |view, _| {
        assert!(view.folder_icon_description().contains("加载失败"));
        assert!(
            view.icon_path(&folder).is_none(),
            "caller keeps official fallback"
        )
    });
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.clear_pending_icons();
            view.queue_type_icons(std::slice::from_ref(&folder), cx);
        })
    });
    draw(&mut surface);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_ok(),
        "original epoch reset permits retry"
    );
}

#[gpui_kit::test]
fn folder_uses_original_pending_type_and_fifteen_second_deadline_budgets(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    let mut rows = (0..12)
        .map(|n| row(&format!("file.ext{n}"), false))
        .collect::<Vec<_>>();
    rows.push(row("folder", true));
    surface.update(|_, cx| view.update(cx, |view, cx| view.queue_type_icons(&rows, cx)));
    draw(&mut surface);
    let mut commands = Vec::new();
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        commands.push(command);
    }
    assert_eq!(
        commands.len(),
        8,
        "folder shares original eight pending slots"
    );
    view.read_with(&surface, |view, _| {
        assert!(view.type_icons.pending.len() <= 8)
    });
    surface
        .executor()
        .advance_clock(std::time::Duration::from_secs(15));
    draw(&mut surface);
    view.read_with(&surface, |view, _| {
        assert!(
            commands
                .iter()
                .all(|command| !view.icon_matches(command.id)),
            "original deadline retires receipts"
        );
        assert_eq!(view.type_icons.failed.len(), 8);
    });
    // Parent Tree rendering may have queued its one folder request after timeout.
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            matches!(&command.message, Message::PlatformCommand(PlatformCommand::ExtractShellIcon(value)) if value.target == "nxtools-folder-type:directory")
        );
    }
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.clear_pending_icons();
            view.type_icons.cache.clear();
            view.type_icons.failed = (0..128).map(|n| format!("ext{n}")).collect();
            view.queue_folder_icon(cx);
        })
    });
    draw(&mut surface);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "folder does not bypass 128 total resources"
    );
}
