use super::*;

fn open_intake(
    cx: &mut TestAppContext,
    application: &Entity<DownloadApplication>,
) -> (AnyWindowHandle, Entity<NewDownloadView>) {
    application.update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    application.read_with(cx, |app, _| {
        (
            app.new_download_window.expect("original intake window"),
            app.new_download_view.clone().expect("original intake view"),
        )
    })
}

fn clipboard_requests(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Vec<uuid::Uuid> {
    std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|envelope| {
            let nxtools_protocol::Message::PlatformCommand(command) = envelope.message else {
                return None;
            };
            (serde_json::to_value(command).expect("typed command")["command"]
                == "read_clipboard_text")
                .then_some(envelope.id)
        })
        .collect()
}

fn reply(
    cx: &mut TestAppContext,
    application: &Entity<DownloadApplication>,
    id: uuid::Uuid,
    success: bool,
    text: Option<&str>,
) {
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: id,
                result: PlatformCommandResult {
                    success,
                    error: (!success).then(|| "clipboard_unavailable".into()),
                    native_error_code: None,
                    data: text.map(str::to_owned),
                },
            },
            cx,
        );
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn download_intake_compact_input_and_right_aligned_actions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let (handle, _view) = open_intake(cx, &application);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("actual compact production window");
    let root = surface.debug_bounds("download-new-root").expect("root");
    assert_eq!(root.size, size(px(560.0), px(136.0)));
    assert!(
        surface
            .debug_bounds("download-new-probe-guidance")
            .is_none()
    );
    assert!(
        surface
            .debug_bounds("download-new-url-field-label")
            .is_none()
    );
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("input");
    let footer = surface.debug_bounds("download-new-footer").expect("footer");
    let next = surface
        .debug_bounds("download-new-probe-action")
        .expect("continue");
    let cancel = surface
        .debug_bounds("download-new-cancel-action")
        .expect("cancel");
    assert!(input.left() <= root.left() + px(16.0));
    assert!(input.bottom() <= footer.top());
    assert!(next.left() > root.center().x);
    assert!(next.right() <= cancel.left());
    assert!((cancel.left() - next.right()) <= px(12.0));
    assert!(next.top() >= footer.top() && next.bottom() <= footer.bottom());
    assert_eq!(footer.bottom(), root.bottom());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_intake_leading_link_slot_has_a_loadable_official_icon(cx: &mut TestAppContext) {
    use gpui_kit::AssetSource as _;

    let path = gpui_kit::assets::IconName::Link.path();
    let bytes = crate::assets::NxToolsAssets
        .load(path.as_ref())
        .expect("the production URL prefix must not reserve an empty icon slot")
        .expect("the production URL prefix SVG is embedded");
    assert!(!bytes.is_empty());
    let renderer = gpui_kit::SvgRenderer::new(std::sync::Arc::new(crate::assets::NxToolsAssets));
    let image = renderer
        .render_single_frame(bytes.as_ref(), 1.0)
        .expect("official Link SVG rasterizes");
    assert!(
        image
            .as_bytes(0)
            .expect("rendered Link pixels")
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0),
        "the leading slot contains visible geometry, not a transparent SVG"
    );
    assert!(
        crate::assets::NxToolsAssets
            .list("icons/")
            .expect("official and product asset enumeration")
            .contains(&path)
    );

    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let (handle, view) = open_intake(cx, &application);
    let input_id = view.read_with(cx, |view, _| view.url.entity_id());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("production URL frame");
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("URL frame");
    surface.simulate_click(
        gpui_kit::point(input.left() + px(12.0), input.center().y),
        Modifiers::default(),
    );
    surface.simulate_keystrokes("h t t p s : / / a . t e s t / x");
    surface.simulate_keystrokes("home");
    surface.simulate_keystrokes("z");
    view.read_with(&surface, |view, cx| {
        assert_eq!(view.url.entity_id(), input_id);
        assert!(view.url.read(cx).value().starts_with('z'));
        assert!(!view.probing && !view.submitting);
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_intake_single_clipboard_link_prefills_once_without_probe(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (handle, view) = open_intake(cx, &application);
    let ids = clipboard_requests(&endpoint);
    assert_eq!(ids.len(), 1, "new manual window reads clipboard once");
    let input_id = view.read_with(cx, |view, _| view.url.entity_id());
    reply(
        cx,
        &application,
        ids[0],
        true,
        Some("下载地址： https://example.test/archive.zip?token=a%2Bb#part"),
    );
    view.read_with(cx, |view, cx| {
        assert_eq!(view.url.entity_id(), input_id);
        assert_eq!(
            view.url.read(cx).value(),
            "https://example.test/archive.zip?token=a%2Bb#part"
        );
        assert!(!view.probing && !view.submitting && view.probe.is_none());
    });
    let (reused, _) = open_intake(cx, &application);
    assert_eq!(reused, handle);
    assert!(clipboard_requests(&endpoint).is_empty());
    assert!(application.read_with(cx, |app, _| app.tasks().is_empty()));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_intake_empty_invalid_and_multiple_clipboard_links_stay_blank(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for text in [
        "",
        "ordinary text",
        "file:///C:/sample.zip",
        "ftp://example.test/file",
        "https://",
        "https://one.test/a https://two.test/b",
        "https://one.test/a\nhttps://two.test/b",
    ] {
        let (application, directory, endpoint) = window_fixture(cx);
        endpoint.set_connected_for_contract_test(true);
        let (_handle, view) = open_intake(cx, &application);
        let ids = clipboard_requests(&endpoint);
        assert_eq!(ids.len(), 1);
        reply(cx, &application, ids[0], true, Some(text));
        view.read_with(cx, |view, cx| {
            assert!(
                view.url.read(cx).value().is_empty(),
                "must not choose a URL from {text:?}"
            );
            assert!(!view.probing && view.error.is_none());
        });
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui_kit::test]
fn download_intake_late_clipboard_never_overwrites_edited_or_cleared_input(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    for clear in [false, true] {
        let (application, directory, endpoint) = window_fixture(cx);
        endpoint.set_connected_for_contract_test(true);
        let (handle, view) = open_intake(cx, &application);
        let ids = clipboard_requests(&endpoint);
        assert_eq!(ids.len(), 1);
        let mut surface = VisualTestContext::from_window(handle, cx);
        view.update_in(&mut surface, |view, window, cx| {
            view.url.update(cx, |input, cx| {
                input.replace_all("https://manual.test/file", window, cx)
            });
        });
        surface.run_until_parked();
        if clear {
            view.update_in(&mut surface, |view, window, cx| {
                view.url
                    .update(cx, |input, cx| input.replace_all("", window, cx));
            });
            surface.run_until_parked();
        }
        reply(
            &mut surface,
            &application,
            ids[0],
            true,
            Some("https://clipboard.test/file"),
        );
        assert_eq!(
            view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string()),
            if clear {
                ""
            } else {
                "https://manual.test/file"
            }
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui_kit::test]
fn download_intake_closed_window_result_cannot_fill_reopened_draft(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (old, _view) = open_intake(cx, &application);
    let first = clipboard_requests(&endpoint);
    assert_eq!(first.len(), 1);
    old.update(cx, |_, window, _| window.remove_window())
        .expect("close original");
    cx.run_until_parked();
    let (new, view) = open_intake(cx, &application);
    assert_ne!(new, old);
    let second = clipboard_requests(&endpoint);
    assert_eq!(second.len(), 1);
    reply(
        cx,
        &application,
        first[0],
        true,
        Some("https://old.test/file"),
    );
    assert!(view.read_with(cx, |view, cx| view.url.read(cx).value().is_empty()));
    reply(
        cx,
        &application,
        second[0],
        true,
        Some("https://new.test/file"),
    );
    assert_eq!(
        view.read_with(cx, |view, cx| view.url.read(cx).value().to_string()),
        "https://new.test/file"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_intake_native_failure_and_disconnect_are_quiet(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for disconnect in [false, true] {
        let (application, directory, endpoint) = window_fixture(cx);
        endpoint.set_connected_for_contract_test(true);
        let (_handle, view) = open_intake(cx, &application);
        let ids = clipboard_requests(&endpoint);
        assert_eq!(ids.len(), 1);
        if disconnect {
            application.update(cx, |app, cx| {
                app.handle_platform_notification(
                    PlatformNotification::Disconnected {
                        generation: 1,
                        pending_command_ids: ids.clone(),
                    },
                    cx,
                )
            });
            cx.run_until_parked();
            reply(
                cx,
                &application,
                ids[0],
                true,
                Some("https://late.test/file"),
            );
        } else {
            reply(cx, &application, ids[0], false, None);
        }
        view.read_with(cx, |view, cx| {
            assert!(view.url.read(cx).value().is_empty());
            assert!(view.error.is_none());
            assert!(!view.probing && !view.submitting);
        });
        assert!(application.read_with(cx, |app, cx| app.notifications.read(cx).items().is_empty()));
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui_kit::test]
fn download_intake_manual_paste_uses_the_original_input_and_does_not_probe(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (handle, view) = open_intake(cx, &application);
    let ids = clipboard_requests(&endpoint);
    assert_eq!(ids.len(), 1);
    reply(cx, &application, ids[0], true, Some(""));
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.update(|_, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
            "  https://manual.test/paste  ".into(),
        ))
    });
    surface.refresh().expect("original compact paste action");
    let paste = surface
        .debug_bounds("download-new-url-paste")
        .expect("paste hit");
    surface.simulate_click(paste.center(), Modifiers::default());
    surface.run_until_parked();
    view.read_with(&surface, |view, cx| {
        assert_eq!(view.url.read(cx).value(), "https://manual.test/paste");
        assert!(!view.probing && !view.submitting);
    });
    assert!(application.read_with(&surface, |app, _| app.tasks().is_empty()));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_intake_timeout_discards_late_clipboard_text(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (_handle, view) = open_intake(cx, &application);
    let ids = clipboard_requests(&endpoint);
    assert_eq!(ids.len(), 1);
    cx.executor().advance_clock(Duration::from_secs(4));
    cx.run_until_parked();
    assert!(application.read_with(cx, |app, _| app.pending_new_download_clipboard.is_none()));
    reply(
        cx,
        &application,
        ids[0],
        true,
        Some("https://late.test/file"),
    );
    assert!(view.read_with(cx, |view, cx| view.url.read(cx).value().is_empty()));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_intake_supplied_request_skips_clipboard_and_preserves_source(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (old, old_view) = open_intake(cx, &application);
    let ids = clipboard_requests(&endpoint);
    assert_eq!(ids.len(), 1);
    let request = old
        .update(cx, |_, window, cx| {
            old_view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.set_value("http://127.0.0.1:9/provided.bin", window, cx)
                });
                view.request(cx).expect("provided request without transfer")
            })
        })
        .expect("original window");
    old.update(cx, |_, window, _| window.remove_window())
        .expect("close original");
    cx.run_until_parked();
    application.update(cx, |app, cx| {
        app.open_new_download_window_with_request(
            Some(NewDownloadInitialSource::Download(request)),
            None,
            cx,
        )
        .expect("supplied source")
    });
    cx.run_until_parked();
    assert!(
        clipboard_requests(&endpoint).is_empty(),
        "provided source must not read clipboard"
    );
    reply(
        cx,
        &application,
        ids[0],
        true,
        Some("https://old.test/clipboard"),
    );
    application.read_with(cx, |app, cx| {
        let view = app
            .new_download_view
            .as_ref()
            .expect("supplied view")
            .read(cx);
        assert_eq!(view.url.read(cx).value(), "http://127.0.0.1:9/provided.bin");
        assert!(view.external_request.is_some());
        assert!(app.tasks().is_empty());
    });
    let _ = std::fs::remove_dir_all(directory);
}
