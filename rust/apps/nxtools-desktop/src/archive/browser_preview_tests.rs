use super::*;
use gpui_kit::{
    Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext,
    VisualTestContext, point, size,
};
use std::{cell::RefCell, rc::Rc};
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn mount(cx: &mut TestAppContext) -> (Entity<ArchiveBrowser>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_,surface)=cx.add_window_view(move |window,cx|{
        let view=cx.new(|cx|{let mut view=ArchiveBrowser::new(PathBuf::from(r"C:\owned\preview.zip"),Uuid::new_v4(),WeakEntity::new_invalid(),window,cx);
            view.generation=1;
            view.install(Uuid::new_v4(),(0..2).map(|ordinal|serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":format!("file-{ordinal}.md"),"kind":"File","size":32,"packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("fixture metadata")).collect(),"Zip".into(),window,cx);
            let first=view.model.as_ref().expect("model").visible()[0].id.clone();
            view.model.as_mut().expect("model").select(first,false,false);
            view
        });*slot.borrow_mut()=Some(view.clone());gpui_kit::base::Root::new(view,window,cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(960.), px(640.)));
    draw(&mut surface);
    (retained.borrow().clone().expect("view"), surface)
}
fn selected(view: &Entity<ArchiveBrowser>, surface: &VisualTestContext) -> Option<RowId> {
    view.read_with(surface, |view, _| {
        view.model
            .as_ref()
            .expect("model")
            .selected
            .iter()
            .next()
            .cloned()
    })
}
fn click_named(surface: &mut VisualTestContext, label: &str) {
    let position = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some(label) && node.role() == Some(Role::Button))
            .expect("official named button")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(surface);
}
#[gpui_kit::test]
fn browser_preview_document_source_and_retirement_stay_local(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    let token = surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let token = view.prepare_preview_for_test(cx);
            view.preview_result(
                token,
                row.clone(),
                PreviewResult::Ready(PreviewPayload::Text {
                    text: "# Heading\n\n**strong** ![blocked](https://example.invalid/a.png)"
                        .into(),
                    mode: DocumentMode::Markdown,
                    budget: Default::default(),
                }),
                window,
                cx,
            );
            token
        })
    });
    draw(&mut surface);
    let document = view.read_with(&surface, |view, _| {
        view.preview
            .document
            .clone()
            .expect("shared passive document")
    });
    document.read_with(&surface, |document, _| {
        assert!(
            document.rendered_text().contains("Heading")
                && document.rendered_text().contains("未加载")
        );
        assert!(!document.rendered_text().contains("https://example.invalid"));
    });
    click_named(&mut surface, "原文");
    view.read_with(&surface, |view, cx| {
        assert!(view.preview.source);
        assert!(
            view.preview
                .document
                .as_ref()
                .expect("source document")
                .read(cx)
                .rendered_text()
                .contains("**strong**")
        );
        assert_eq!(view.preview.generation, token);
        assert!(view.open_dispatches.is_empty());
    });
    surface.update(|_, cx| view.update(cx, |view, cx| view.retire(cx)));
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.preview_result(
                token,
                row,
                PreviewResult::Ready(PreviewPayload::Text {
                    text: "stale".into(),
                    mode: DocumentMode::Markdown,
                    budget: Default::default(),
                }),
                window,
                cx,
            )
        })
    });
    assert!(view.read_with(&surface, |view, _| view.preview.document.is_none()));
}
#[gpui_kit::test]
fn browser_preview_same_turn_selection_aba_rejects_old_result_and_preserves_doubleclick_owner(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    let token =
        surface.update(|_, cx| view.update(cx, |view, cx| view.prepare_preview_for_test(cx)));
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let second = view.model.as_ref().expect("model").visible()[1].id.clone();
            view.model
                .as_mut()
                .expect("model")
                .select(second, false, false);
            view.update_summary();
            view.model
                .as_mut()
                .expect("model")
                .select(row.clone().expect("single"), false, false);
            view.update_summary();
            view.preview_result(
                token,
                row.clone(),
                PreviewResult::Ready(PreviewPayload::Text {
                    text: "old".into(),
                    mode: DocumentMode::Markdown,
                    budget: Default::default(),
                }),
                window,
                cx,
            );
            assert!(view.preview.document.is_none());
            assert!(view.preview.generation > token);
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |view, _| assert!(view.open_dispatches.is_empty()));
}
#[gpui_kit::test]
fn browser_preview_minimum_themes_and_actual_splitter_keep_list_and_actions_reachable(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.prepare_preview_for_test(cx);
        })
    });
    draw(&mut surface);
    let before = surface
        .debug_bounds("archive-preview-pane")
        .expect("preview");
    let start = point(before.left(), before.center().y);
    surface.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    for distance in [8., 24., 48.] {
        surface.simulate_event(MouseMoveEvent {
            position: point(start.x - px(distance), start.y),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(&mut surface);
    }
    surface.simulate_event(MouseUpEvent {
        position: point(start.x - px(48.), start.y),
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    let after = surface
        .debug_bounds("archive-preview-pane")
        .expect("resized");
    assert!(
        after.size.width > before.size.width + px(8.),
        "real official preview handle must resize: {before:?}->{after:?}"
    );
    for theme in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        surface.update(|_, cx| crate::theme::set_theme_preference(theme, cx));
        surface.simulate_resize(size(px(720.), px(480.)));
        draw(&mut surface);
        let preview = surface
            .debug_bounds("archive-preview-pane")
            .expect("minimum preview");
        let list = surface.debug_bounds("archive-horizontal").expect("list");
        assert!(
            preview.right() <= px(720.)
                && preview.size.width >= px(200.)
                && list.size.width >= px(158.),
            "minimum bounds: {preview:?}/{list:?}"
        );
    }
}

#[gpui_kit::test]
fn browser_preview_pointer_keyboard_selection_share_one_generation_owner(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    let original =
        surface.update(|_, cx| view.update(cx, |view, cx| view.prepare_preview_for_test(cx)));
    draw(&mut surface);
    let position = surface
        .debug_bounds("archive-entry-1")
        .expect("production row")
        .center();
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    let pointer = view.read_with(&surface, |view, _| {
        let row = view
            .model
            .as_ref()
            .expect("model")
            .selected
            .iter()
            .next()
            .cloned();
        assert!(matches!(row, Some(RowId::Entry(_, 1))));
        assert!(view.preview_selection_matches(view.preview.generation, &row));
        assert!(view.preview.generation > original);
        view.preview.generation
    });
    surface.simulate_event(KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("up").expect("Up"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("up").expect("release"),
    });
    draw(&mut surface);
    view.read_with(&surface, |view, _| {
        let row = view
            .model
            .as_ref()
            .expect("model")
            .selected
            .iter()
            .next()
            .cloned();
        assert!(matches!(row, Some(RowId::Entry(_, 0))));
        assert!(view.preview_selection_matches(view.preview.generation, &row));
        assert!(view.preview.generation > pointer);
        assert!(view.open_dispatches.is_empty());
    });
}

#[gpui_kit::test]
fn browser_preview_long_document_last_block_scrolls_inside_its_pane(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    let source = format!(
        "# Heading\n\n{}\n\nLAST-BLOCK",
        "paragraph **strong**\n\n".repeat(80)
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let token = view.prepare_preview_for_test(cx);
            view.preview_result(
                token,
                row,
                PreviewResult::Ready(PreviewPayload::Text {
                    text: source,
                    mode: DocumentMode::Markdown,
                    budget: Default::default(),
                }),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.preview.scroll.set_offset(point(px(0.), px(-100000.)));
            cx.notify();
        })
    });
    for _ in 0..4 {
        draw(&mut surface);
    }
    view.read_with(&surface, |view, cx| {
        let document = view.preview.document.as_ref().expect("document").read(cx);
        assert!(document.rendered_text().contains("LAST-BLOCK"));
        let viewport = view.preview.scroll.bounds();
        assert!(view.preview.scroll.offset().y < px(0.));
        assert!(
            document.bounds().bottom() <= viewport.bottom() + px(1.),
            "last document block reachable: {:?}/{viewport:?}",
            document.bounds()
        );
    });
}

#[gpui_kit::test]
fn browser_preview_owned_image_has_no_path_and_fits_minimum_pane(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    let image = std::sync::Arc::new(gpui_kit::RenderImage::new(vec![image::Frame::new(
        image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 0, 255, 255])),
    )]));
    let retained = std::sync::Arc::downgrade(&image);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let token = view.prepare_preview_for_test(cx);
            view.preview_result(
                token,
                row,
                PreviewResult::Ready(PreviewPayload::Image {
                    image,
                    width: 2,
                    height: 1,
                }),
                window,
                cx,
            );
        })
    });
    surface.simulate_resize(size(px(720.), px(480.)));
    draw(&mut surface);
    let pane = surface.debug_bounds("archive-preview-pane").expect("pane");
    let image = surface
        .debug_bounds("archive-preview-image")
        .expect("owned image renderer");
    assert!(
        pane.contains(&image.center())
            && image.right() <= pane.right()
            && image.bottom() <= pane.bottom()
    );
    surface.update(|_, cx| view.update(cx, |view, cx| view.suspend_tab(cx)));
    draw(&mut surface);
    assert!(
        retained.upgrade().is_none(),
        "closed view releases its owned image, no temporary file required"
    );
}

#[gpui_kit::test]
fn feedback_preview_original_decoder_to_actual_md_txt_image_pane(cx: &mut TestAppContext) {
    use crate::archive::preview::{self, PreviewKind};
    use image::ImageEncoder as _;
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    let directory = tempfile::tempdir().expect("owned preview input");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&[255, 0, 0, 255], 1, 1, image::ExtendedColorType::Rgba8)
        .expect("tiny PNG");
    for (name, bytes, kind) in [
        (
            "readme.md",
            b"# Visible heading\n\n![blocked](https://example.invalid/private.png)".to_vec(),
            PreviewKind::Text(DocumentMode::Markdown),
        ),
        (
            "plain.txt",
            b"**literal, not bold**\r\nlast line".to_vec(),
            PreviewKind::Text(DocumentMode::Literal(None)),
        ),
        ("image.png", png, PreviewKind::Image),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, bytes).expect("owned bounded input");
        let payload = preview::prepare(&path, kind, Default::default(), &std::sync::atomic::AtomicBool::new(false))
            .unwrap_or_else(|_| panic!("original decoder: {name}"));
        let token = view.read_with(&surface, |view, _| view.preview.generation);
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.preview_result(
                    token,
                    row.clone(),
                    PreviewResult::Ready(payload),
                    window,
                    cx,
                )
            })
        });
        draw(&mut surface);
        assert!(
            surface.debug_bounds("archive-preview-pane").is_some(),
            "decoded {name} must reach the actual pane without opt-in fixture setup"
        );
        view.read_with(&surface, |view, cx| {
            if name.ends_with(".png") {
                assert!(matches!(
                    view.preview.result,
                    PreviewResult::Ready(PreviewPayload::Image { .. })
                ));
            } else {
                let document = view
                    .preview
                    .document
                    .as_ref()
                    .expect("actual shared document")
                    .read(cx);
                assert!(document.editor.read(cx).is_readonly());
                assert!(document.rendered_text().contains(if name.ends_with(".md") {
                    "Visible heading"
                } else {
                    "**literal, not bold**"
                }));
                assert!(!document.rendered_text().contains("https://example.invalid"));
            }
            assert!(view.open_dispatches.is_empty());
        });
    }
}

#[gpui_kit::test]
fn feedback_preview_long_markdown_real_wheel_reaches_content(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let token = view.prepare_preview_for_test(cx);
            view.preview_result(
                token,
                row,
                PreviewResult::Ready(PreviewPayload::Text {
                    text: format!(
                        "# Heading\n\n{}\n\nEND",
                        "paragraph **strong**\n\n".repeat(80)
                    ),
                    mode: DocumentMode::Markdown,
                    budget: Default::default(),
                }),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    let viewport = surface
        .debug_bounds("archive-preview-document-scroll")
        .expect("actual document scroll viewport");
    surface.simulate_event(gpui_kit::ScrollWheelEvent {
        position: viewport.center(),
        delta: gpui_kit::ScrollDelta::Lines(point(0., -4.)),
        ..Default::default()
    });
    draw(&mut surface);
    assert!(
        view.read_with(&surface, |view, _| view.preview.scroll.offset().y < px(0.)),
        "real wheel must scroll the actual passive document, not just a programmatic offset"
    );
}

#[gpui_kit::test]
fn feedback_preview_actual_state_matrix_retains_retry_and_never_opens_externally(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    for state in [
        PreviewResult::Idle,
        PreviewResult::MultipleSelection,
        PreviewResult::Loading,
        PreviewResult::Unsupported,
        PreviewResult::TooLarge,
        PreviewResult::ReadLimit,
        PreviewResult::TextLines,
        PreviewResult::ImageDimensions,
        PreviewResult::ImageMemory,
        PreviewResult::DocumentLimit,
        PreviewResult::Directory,
        PreviewResult::PasswordCancelled,
        PreviewResult::SourceChanged,
        PreviewResult::Disconnected,
        PreviewResult::Corrupt,
        PreviewResult::Failed,
        PreviewResult::Cancelled,
    ] {
        let expected_title = match &state {
            PreviewResult::TooLarge => Some("文件超过预览读取大小上限"),
            PreviewResult::ReadLimit => Some("压缩包读取资源达到上限"),
            PreviewResult::TextLines => Some("文本超过预览行数上限"),
            PreviewResult::ImageDimensions => Some("图片尺寸超过预览上限"),
            PreviewResult::ImageMemory => Some("图片解码内存超过预览上限"),
            PreviewResult::DocumentLimit => Some("文档超过预览呈现上限"),
            _ => None,
        };
        let retry = matches!(
            state,
            PreviewResult::PasswordCancelled
                | PreviewResult::Corrupt
                | PreviewResult::Failed
                | PreviewResult::Cancelled
        );
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                let token = view.prepare_preview_for_test(cx);
                view.preview_result(token, row.clone(), state, window, cx);
            })
        });
        draw(&mut surface);
        assert!(surface.debug_bounds("archive-preview-pane").is_some());
        if let Some(title) = expected_title {
            assert!(
                surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .any(|node| node.label() == Some(title) && node.bounds().size.width > px(0.))),
                "actual stage-specific preview title: {title}"
            );
        }
        assert_eq!(
            surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some("重试") && node.role() == Some(Role::Button))),
            retry
        );
        assert!(
            view.read_with(&surface, |view, _| view.preview.document.is_none()
                && view.open_dispatches.is_empty())
        );
    }
}

#[gpui_kit::test]
fn feedback_preview_tab_suspend_retires_local_content_and_old_selection_token(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    let row = selected(&view, &surface);
    let token = surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let token = view.prepare_preview_for_test(cx);
            view.preview_result(
                token,
                row.clone(),
                PreviewResult::Ready(PreviewPayload::Text {
                    text: "old tab".into(),
                    mode: DocumentMode::Literal(None),
                    budget: Default::default(),
                }),
                window,
                cx,
            );
            view.suspend_tab(cx);
            assert!(
                !view.preview_selection_matches(token, &row),
                "hidden tab must retire its old preview token immediately"
            );
            assert!(
                view.preview.document.is_none(),
                "hidden tab drops local content"
            );
            token
        })
    });
    draw(&mut surface);
    assert!(
        view.read_with(&surface, |view, _| view.preview.generation > token),
        "returning render refreshes current automatic selection"
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let model = view.model.as_mut().expect("original selection owner");
            model.selected.clear();
            view.update_summary();
            view.sync_preview_selection(cx);
            assert!(matches!(view.preview.result, PreviewResult::Idle));
            let rows = view
                .model
                .as_ref()
                .expect("model")
                .visible()
                .iter()
                .map(|row| row.id.clone())
                .collect::<Vec<_>>();
            view.model.as_mut().expect("model").selected.extend(rows);
            view.update_summary();
            view.sync_preview_selection(cx);
            assert!(matches!(
                view.preview.result,
                PreviewResult::MultipleSelection
            ));
            view.set_failure("源已变化".into(), window, cx);
            view.sync_preview_selection(cx);
            assert!(!view.preview_selection_matches(token, &row));
            assert!(view.preview.document.is_none());
        })
    });
}

#[gpui_kit::test]
fn automatic_preview_ignores_historical_disabled_state_and_has_no_opt_in_controls(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let mut legacy = serde_json::to_value(view.snapshot_from_observed()).expect("snapshot");
            legacy["show_preview"] = false.into();
            let state = serde_json::from_value(legacy).expect("legacy state");
            assert!(view.apply_restored_state(state, window, cx));
            view.list_focus.focus(window, cx);
        })
    });
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-preview-pane").is_some(),
        "old false must not hide automatic preview"
    );
    surface.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert!(
            !nodes.iter().any(|node| node.role() == Some(Role::Button)
                && matches!(node.label(), Some("预览" | "关闭预览栏"))),
            "automatic preview has no enable/disable controls"
        );
    });
    let row = selected(&view, &surface);
    let token = view.read_with(&surface, |view, _| view.preview.generation);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.preview_result(
                token,
                row.clone(),
                PreviewResult::Ready(PreviewPayload::Text {
                    text: "automatic small text".into(),
                    mode: DocumentMode::Literal(None),
                    budget: Default::default(),
                }),
                window,
                cx,
            );
            assert!(view.list_focus.is_focused(window));
            assert!(view.preview.document.is_some());
            view.suspend_tab(cx);
            view.preview_result(
                token,
                row,
                PreviewResult::Ready(PreviewPayload::Text {
                    text: "stale".into(),
                    mode: DocumentMode::Literal(None),
                    budget: Default::default(),
                }),
                window,
                cx,
            );
            assert!(view.preview.document.is_none());
        })
    });
}

#[gpui_kit::test]
fn runtime_feedback_preview_pane_only_occupies_space_for_supported_single_file(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.model.as_mut().expect("model").selected.clear();
            view.update_summary();
            view.sync_preview_selection(cx);
            cx.notify();
        })
    });
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-preview-pane").is_none(),
        "opening an archive without a supported selection must not reserve a right pane"
    );
    let collapsed = surface
        .debug_bounds("archive-horizontal")
        .expect("list viewport");
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            let model = view.model.as_mut().expect("model");
            model.select(RowId::Entry(model.snapshot, 0), false, false);
            view.update_summary();
            view.sync_preview_selection(cx);
            cx.notify();
        })
    });
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-preview-pane").is_some(),
        "supported selection automatically reveals the existing preview"
    );
    let expanded = surface
        .debug_bounds("archive-horizontal")
        .expect("list viewport");
    assert!(
        expanded.size.width < collapsed.size.width,
        "hidden preview must release its actual layout width"
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.install(
                Uuid::new_v4(),
                vec![serde_json::from_value(serde_json::json!({
            "ordinal":0,"name":"unsupported.bin","kind":"File","size":80,"packed_size":null,
            "encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001
        })).expect("unsupported metadata")],
                "Zip".into(),
                window,
                cx,
            );
            let model = view.model.as_mut().expect("model");
            model.select(RowId::Entry(model.snapshot, 0), false, false);
            view.update_summary();
            view.sync_preview_selection(cx);
            cx.notify();
        })
    });
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-preview-pane").is_none(),
        "unsupported files never reserve or retain the preview pane"
    );
}
