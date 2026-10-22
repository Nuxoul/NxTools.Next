//! Isolated TestWindow automation over the production form; never native/GUI acceptance.
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext, size};
use std::{cell::RefCell, rc::Rc};

fn source() -> DraftSource {
    DraftSource {
        path: r"C:\synthetic\input.txt".into(),
        directory: false,
        bytes: Some(12),
    }
}
fn mount(
    cx: &mut TestAppContext,
    settings: ArchiveSettings,
) -> (Entity<OperationForm>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let capture = Rc::new(RefCell::new(None));
    let slot = capture.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            OperationForm::create(
                settings,
                vec![source()],
                WeakEntity::new_invalid(),
                window,
                cx,
            )
        });
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(680.), px(560.)));
    draw(&mut surface);
    (capture.borrow().clone().expect("production form"), surface)
}
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn click(surface: &mut VisualTestContext, selector: &'static str) {
    draw(surface);
    let position = surface
        .debug_bounds(selector)
        .expect("current real Button")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(surface);
}
#[gpui_kit::test]
fn saved_defaults_real_inputs_and_request_are_one_snapshot(cx: &mut TestAppContext) {
    let settings = ArchiveSettings {
        create_format: ArchiveCreateFormat::SevenZ,
        compression_level: ArchiveCompressionLevel::Maximum,
        test_after_create: false,
        ..Default::default()
    };
    let (view, mut surface) = mount(cx, settings);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            let (requests, completion) = form.capture(cx).expect("valid saved draft");
            let create = requests[0].create.as_ref().expect("create");
            assert_eq!(create.format, ArchiveJobFormat::SevenZ);
            assert_eq!(create.level, Some(ArchiveJobLevel::Maximum));
            assert!(!create.test_after_create);
            assert!(completion.remember.is_none());
            assert_eq!(form.name.read(cx).value().as_ref(), "input.7z");
        })
    });
}
#[gpui_kit::test]
fn real_format_callback_defers_and_requires_explicit_secret_discard(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| view.update(cx, |form, cx| {
        form.password.update(cx, |input, cx| input.set_value("private-sentinel", window, cx));
        form.confirm.update(cx, |input, cx| input.set_value("private-sentinel", window, cx));
        form.format.update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("TAR"))));
        assert!(matches!(&form.draft, Draft::Create(draft) if draft.format == ArchiveCreateFormat::Zip));
    }));
    draw(&mut surface);
    view.read_with(&surface, |form, cx| {
        assert_eq!(form.pending_format, Some(ArchiveCreateFormat::Tar));
        assert!(!form.password.read(cx).value().is_empty());
    });
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    surface.simulate_keystrokes("escape");
    draw(&mut surface);
    view.read_with(&surface, |form, cx| {
        assert!(form.pending_format.is_none());
        assert!(!form.password.read(cx).value().is_empty());
        assert_eq!(form.format.read(cx).selected_value(), Some(&"ZIP"));
    });
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            form.change_format(ArchiveCreateFormat::Tar, window, cx)
        })
    });
    draw(&mut surface);
    click(&mut surface, "archive-discard-secret");
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            let (requests, _) = form.capture(cx).expect("confirmed TAR");
            let create = requests[0].create.as_ref().expect("create");
            assert!(create.password.is_none() && create.level.is_none() && create.method.is_none());
            assert!(form.password.read(cx).value().is_empty());
        })
    });
}
#[gpui_kit::test]
fn minimum_footer_is_fixed_medium_and_entities_survive_repaint(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let original = view.read_with(&surface, |form, _| {
        (form.name.entity_id(), form.format.entity_id())
    });
    surface.simulate_resize(size(px(480.), px(360.)));
    draw(&mut surface);
    for selector in ["archive-form-submit", "archive-form-cancel"] {
        let bounds = surface.debug_bounds(selector).expect("real footer Button");
        assert_eq!(bounds.size.height, px(32.));
        assert!(bounds.top() >= px(0.) && bounds.bottom() <= px(360.));
        assert!(bounds.left() >= px(0.) && bounds.right() <= px(480.));
    }
    view.read_with(&surface, |form, _| {
        assert_eq!(original, (form.name.entity_id(), form.format.entity_id()))
    });
}
#[gpui_kit::test]
fn picker_requires_exact_operation_session_generation_and_is_once_only(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            let operation = Uuid::new_v4();
            form.pending_picker = Some(PendingPicker {
                operation: Some(operation),
                purpose: ArchivePickerPurpose::Destination,
            });
            let mut result = ArchivePickerResult {
                version: 1,
                operation_id: operation,
                session_id: form.id,
                generation: form.generation + 1,
                paths: vec![r"C:\synthetic\chosen".into()],
                cancelled: false,
                code: None,
                native_error_code: None,
            };
            form.picker_result(result.clone(), window, cx);
            assert!(form.pending_picker.is_some());
            result.generation = form.generation;
            result.operation_id = Uuid::new_v4();
            form.picker_result(result.clone(), window, cx);
            assert!(form.pending_picker.is_some());
            result.operation_id = operation;
            form.picker_result(result.clone(), window, cx);
            assert_eq!(
                form.destination.read(cx).value().as_ref(),
                r"C:\synthetic\chosen"
            );
            assert!(form.pending_picker.is_none());
            result.paths = vec![r"C:\synthetic\wrong".into()];
            form.picker_result(result, window, cx);
            assert_eq!(
                form.destination.read(cx).value().as_ref(),
                r"C:\synthetic\chosen"
            );
        })
    });
}
#[gpui_kit::test]
fn submit_snapshots_once_and_rejection_retains_draft(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            form.submit(window, cx);
            form.submit(window, cx);
            assert_eq!(form.submit_count, 1);
            assert!(form.submitted);
            form.fail_submit("native admission rejected".into(), window, cx);
            assert!(!form.submitted && !form.retired);
            assert_eq!(form.name.read(cx).value().as_ref(), "input.zip");
        })
    });
}
#[gpui_kit::test]
fn invalid_password_blocks_request_and_retirement_clears_secrets(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            form.password
                .update(cx, |input, cx| input.set_value("one", window, cx));
            form.confirm
                .update(cx, |input, cx| input.set_value("two", window, cx));
            form.submit(window, cx);
            assert_eq!(form.submit_count, 0);
            assert!(form.error.is_some());
            form.retire(window, cx);
            assert!(
                form.password.read(cx).value().is_empty()
                    && form.confirm.read(cx).value().is_empty()
            );
            assert!(form.retired);
            form.submit(window, cx);
            assert_eq!(form.submit_count, 0);
        })
    });
}

#[gpui_kit::test]
fn production_separate_checkbox_captures_complete_batch_and_saved_defaults_without_secrets(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            if let Draft::Create(draft) = &mut form.draft {
                draft.sources.push(DraftSource {
                    path: r"C:\synthetic\second.txt".into(),
                    directory: false,
                    bytes: Some(1),
                });
            }
            form.apply_format(ArchiveCreateFormat::Xz, window, cx);
            assert!(
                form.capture(cx).is_err(),
                "streams cannot silently drop the second input"
            );
            form.flag(Flag::Separately, true, cx);
            form.flag(Flag::Remember, true, cx);
            let (requests, options) = form.capture(cx).expect("complete explicit batch");
            assert_eq!(requests.len(), 2);
            assert_eq!(requests[0].paths, vec![r"C:\synthetic\input.txt"]);
            assert_eq!(requests[1].paths, vec![r"C:\synthetic\second.txt"]);
            let saved = options.remember.expect("explicit remember");
            assert_eq!(saved.create_format, ArchiveCreateFormat::Xz);
            assert_eq!(saved.exclude_patterns, vec!["Thumbs.db", "desktop.ini"]);
        })
    });
}
#[gpui_kit::test]
fn oversized_picker_is_all_or_nothing_and_cancel_never_changes_destination(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            let operation = Uuid::new_v4();
            form.pending_picker = Some(PendingPicker {
                operation: Some(operation),
                purpose: ArchivePickerPurpose::Files,
            });
            let result = ArchivePickerResult {
                version: 1,
                operation_id: operation,
                session_id: form.id,
                generation: form.generation,
                paths: (0..33).map(|n| format!(r"C:\synthetic\{n}.txt")).collect(),
                cancelled: false,
                code: None,
                native_error_code: None,
            };
            form.picker_result(result, window, cx);
            assert!(form.error.is_some() && !form.enumerating);
            assert!(matches!(&form.draft,Draft::Create(draft) if draft.sources.len()==1));
            form.pending_picker = Some(PendingPicker {
                operation: Some(operation),
                purpose: ArchivePickerPurpose::Destination,
            });
            form.picker_result(
                ArchivePickerResult {
                    version: 1,
                    operation_id: operation,
                    session_id: form.id,
                    generation: form.generation,
                    paths: vec![],
                    cancelled: true,
                    code: None,
                    native_error_code: None,
                },
                window,
                cx,
            );
            assert_eq!(form.destination.read(cx).value().as_ref(), r"C:\synthetic");
            assert!(!form.remember && !form.submitted);
        })
    });
}
#[gpui_kit::test]
fn production_extract_form_uses_saved_strategy_conflict_and_disabled_empty_selection(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window,cx|view.update(cx,|form,cx| {
        let settings=ArchiveSettings {default_destination:ArchiveDestination::Name,conflict_policy:ArchiveConflictPolicy::Skip,copy_motw:false,..Default::default()};
        let mut extract=OperationForm::extract(settings,r"C:\synthetic\Photos.tar.gz".into(),None,WeakEntity::new_invalid(),window,cx);
        let (requests,completion)=extract.capture(cx).expect("extract form request");
        let request=requests[0].extract.as_ref().expect("extract options");
        assert_eq!(request.strategy,ArchiveJobStrategy::Name);
        assert_eq!(request.conflict,ArchiveJobConflict::Skip);
        assert!(request.selection.is_none()&&!request.copy_motw);
        assert!(completion.open_after&&!completion.keep_open&&completion.remember.is_none());
        assert!(matches!(&extract.draft,Draft::Extract(draft) if !draft.selected_only&&!draft.delete_source));
        // Replace the production state only in the isolated fixture, preserving its stable Root.
        *form=extract;
    }));
    surface.simulate_resize(size(px(440.), px(360.)));
    draw(&mut surface);
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(nodes.iter().any(|node| node.label() == Some("选定文件")));
    assert!(
        surface
            .debug_bounds("archive-form-submit")
            .expect("extract footer")
            .bottom()
            <= px(360.)
    );
    // Disabled radio must not change the real selected range through a pointer action.
    let position = nodes
        .iter()
        .find(|node| node.label() == Some("选定文件"))
        .expect("disabled selection radio")
        .bounds()
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(&mut surface);
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft,Draft::Extract(draft) if !draft.selected_only))
    });
}

#[test]
fn metadata_worker_is_bounded_cancellable_and_does_not_recurse() {
    let root = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("archive-form-metadata-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).expect("unique isolated source root");
    let file = root.join("source.txt");
    std::fs::write(&file, b"fixture").expect("tiny isolated source");
    let cancel = AtomicBool::new(false);
    let paths = vec![
        root.to_str().expect("Unicode fixture").to_owned(),
        file.to_str().expect("Unicode file").to_owned(),
    ];
    let sources = source_metadata(paths.clone(), &cancel).expect("root metadata only");
    assert_eq!(sources.len(), 2);
    assert!(sources[0].directory && sources[0].bytes.is_none());
    assert_eq!(sources[1].bytes, Some(7));
    assert!(source_metadata(vec![paths[1].clone(); 33], &cancel).is_err());
    cancel.store(true, Ordering::Release);
    assert!(source_metadata(paths, &cancel).is_err());
    std::fs::remove_file(file).expect("owned fixture file cleanup");
    std::fs::remove_dir(root).expect("owned empty fixture root cleanup");
}
#[gpui_kit::test]
fn production_split_field_stable_entity_and_checked_wire(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let entity = view.read_with(&surface, |form, _| form.volume.entity_id());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            form.volume
                .update(cx, |input, cx| input.set_value("128", window, cx));
            let (requests, _) = form.capture(cx).expect("split form");
            assert_eq!(
                requests[0].create.as_ref().expect("create").volume_bytes,
                Some(128 << 20)
            );
        });
    });
    draw(&mut surface);
    view.read_with(&surface, |form, _| {
        assert_eq!(form.volume.entity_id(), entity)
    });
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("分卷大小 MiB"))
    }));
}
#[gpui_kit::test]
fn production_modify_form_retains_draft_on_failure_and_retires_old_picker(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            let draft = ModifyDraft {
                source: r"C:\synthetic\fixture.zip".into(),
                browser: ArchiveBrowserContext {
                    snapshot_id: Uuid::new_v4(),
                    format: "Zip".into(),
                    comment: String::new(),
                    source: ArchiveSourceBinding {
                        native: "v1:fixture".into(),
                        digest: [1; 32],
                        size: 1,
                        modified_unix_ns: 1,
                    },
                },
                action: ArchiveJobModifyAction::Rename,
                ordinals: vec![1],
                value: Some("old.txt".into()),
                added: vec![],
                codepage: 0,
            };
            *form = OperationForm::modify(
                ArchiveSettings::default(),
                draft,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            let entity = form.name.entity_id();
            form.name
                .update(cx, |input, cx| input.set_value("new.txt", window, cx));
            assert_eq!(
                form.capture(cx).expect("valid rename").0[0]
                    .modify
                    .as_ref()
                    .expect("edit")
                    .value
                    .as_deref(),
                Some("new.txt")
            );
            form.submit(window, cx);
            assert!(form.submitted);
            form.fail_submit("native rejected".into(), window, cx);
            assert_eq!(form.name.entity_id(), entity);
            assert_eq!(form.name.read(cx).value().as_ref(), "new.txt");
            let old = Uuid::new_v4();
            form.pending_picker = Some(PendingPicker {
                operation: Some(old),
                purpose: ArchivePickerPurpose::Files,
            });
            form.retire(window, cx);
            form.picker_result(
                ArchivePickerResult {
                    version: 1,
                    operation_id: old,
                    session_id: form.id,
                    generation: form.generation - 1,
                    paths: vec![r"C:\synthetic\wrong.txt".into()],
                    cancelled: false,
                    code: None,
                    native_error_code: None,
                },
                window,
                cx,
            );
            assert!(form.retired && form.pending_picker.is_none());
        });
    });
    draw(&mut surface);
}
