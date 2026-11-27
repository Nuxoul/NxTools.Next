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
fn production_extract_form_uses_saved_strategy_conflict_without_empty_selection_action(
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
    assert!(!nodes.iter().any(|node| {
        node.label()
            .is_some_and(|label| label.starts_with("选定文件"))
    }));
    assert!(
        surface
            .debug_bounds("archive-form-submit")
            .expect("extract footer")
            .bottom()
            <= px(360.)
    );
    // With no browser selection, the form exposes no selected-range action.
    assert!(surface.debug_bounds("archive-extract-range").is_none());
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
    let directory = root.join("directory");
    std::fs::create_dir(&directory).expect("owned directory source");
    let cancel = AtomicBool::new(false);
    let paths = vec![
        directory.to_str().expect("Unicode fixture").to_owned(),
        file.to_str().expect("Unicode file").to_owned(),
    ];
    let sources = source_metadata(paths.clone(), &cancel).expect("root metadata only");
    assert_eq!(sources.len(), 2);
    assert!(sources[0].directory && sources[0].bytes.is_none());
    assert_eq!(sources[1].bytes, Some(7));
    assert!(
        source_metadata(
            vec![root.to_str().expect("root").to_owned(), paths[1].clone()],
            &cancel
        )
        .is_err(),
        "overlapping roots refused atomically"
    );
    assert!(source_metadata(vec![paths[1].clone(); 33], &cancel).is_err());
    cancel.store(true, Ordering::Release);
    assert!(source_metadata(paths, &cancel).is_err());
    std::fs::remove_file(file).expect("owned fixture file cleanup");
    std::fs::remove_dir(directory).expect("owned source directory cleanup");
    std::fs::remove_dir(root).expect("owned empty fixture root cleanup");
}
#[gpui_kit::test]
fn production_split_field_stable_entity_and_checked_wire(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, "archive-options-toggle");
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
            .any(|node| node.label() == Some("分卷大小数值"))
    }));
}
#[gpui_kit::test]
fn create_invalid_options_are_disabled_and_format_switch_preserves_volume(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, "archive-options-toggle");
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.format
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("7Z"))));
        });
    });
    draw(&mut surface);
    let header_disabled = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("加密文件名（需要密码）"))
            .and_then(|node| node.disabled())
    });
    assert_eq!(header_disabled, Some(true));
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            for input in [&form.password, &form.confirm] {
                input.update(cx, |input, cx| input.set_value("confirmed", window, cx));
            }
        });
    });
    draw(&mut surface);
    let header_disabled = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("加密文件名（需要密码）"))
            .expect("rendered filename-encryption checkbox")
            .disabled()
            .unwrap_or(false)
    });
    assert!(!header_disabled);
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            for input in [&form.password, &form.confirm] {
                input.update(cx, |input, cx| input.set_value("", window, cx));
            }
            form.volume
                .update(cx, |input, cx| input.set_value("128", window, cx));
            form.format
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("TAR"))));
        });
    });
    draw(&mut surface);
    view.read_with(&surface, |form, cx| {
        assert_eq!(form.volume.read(cx).value().as_ref(), "128");
        assert!(matches!(&form.draft, Draft::Create(draft) if draft.format == ArchiveCreateFormat::Tar && draft.volume_mib == "128"));
    });
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("分卷大小数值"))
    }));
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            assert!(
                form.capture(cx).is_err(),
                "unsupported retained split requires explicit none"
            );
            form.volume_preset
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("不分卷"))));
        });
    });
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            let (requests, _) = form.capture(cx).expect("TAR after explicit no split");
            assert_eq!(
                requests[0].create.as_ref().expect("create").volume_bytes,
                None
            );
        })
    });
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
                    sevenz_edit: None,
                    tar_edit: None,
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
                source_prefix: None,
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

#[gpui_kit::test]
fn saved_completion_preferences_and_lossless_rules_reach_real_capture_and_remember(
    cx: &mut TestAppContext,
) {
    let patterns = vec![
        " keep spaces ".to_owned(),
        "semi;colon".into(),
        "line\nbreak".into(),
    ];
    let settings = ArchiveSettings {
        open_after_extract: false,
        keep_extract_progress_open: true,
        smart_single_file_folder: true,
        keep_create_progress_open: true,
        exclude_patterns: patterns.clone(),
        ..Default::default()
    };
    let (view, mut surface) = mount(cx, settings.clone());
    view.update_in(&mut surface, |form, window, cx| {
        let (requests, completion) = form.capture(cx).expect("lossless creation");
        assert_eq!(
            requests[0]
                .create
                .as_ref()
                .expect("create")
                .exclude_patterns,
            patterns
        );
        assert!(completion.keep_open);
        form.remember = true;
        form.keep_open = false;
        let (_, completion) = form.capture(cx).expect("remember create");
        let remembered = completion.remember.expect("create defaults");
        assert!(!remembered.keep_create_progress_open);
        assert!(remembered.keep_extract_progress_open);
        assert_eq!(remembered.exclude_patterns, patterns);
        let mut extract = OperationForm::extract(
            settings,
            r"C:\synthetic\archive.zip".into(),
            None,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        let (_, completion) = extract.capture(cx).expect("saved extract");
        assert!(!completion.open_after && completion.keep_open);
        assert!(matches!(&extract.draft, Draft::Extract(draft) if draft.single_file_folder));
        extract.remember = true;
        extract.open_after = true;
        extract.keep_open = false;
        if let Draft::Extract(draft) = &mut extract.draft {
            draft.single_file_folder = false;
        }
        let (_, completion) = extract.capture(cx).expect("remember extract");
        let remembered = completion.remember.expect("extract defaults");
        assert!(remembered.open_after_extract);
        assert!(!remembered.keep_extract_progress_open && !remembered.smart_single_file_folder);
        assert!(remembered.keep_create_progress_open);
    });
}

#[gpui_kit::test]
fn create_rule_buttons_preserve_individual_rules_and_invalid_draft(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, "archive-options-toggle");
    for _ in 0..30 {
        draw(&mut surface);
        if surface
            .debug_bounds("archive-create-rule-add")
            .is_some_and(|bounds| bounds.bottom() <= px(510.))
        {
            break;
        }
        surface.simulate_event(gpui_kit::ScrollEvent {
            position: gpui_kit::point(px(600.), px(450.)),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., -4.)),
            ..Default::default()
        });
    }
    click(&mut surface, "archive-create-rule-add");
    assert_eq!(
        view.read_with(&surface, |form, _| form.create_exclusions.len()),
        3
    );
    view.update_in(&mut surface, |form, window, cx| {
        form.create_exclusions[2].update(cx, |state, cx| state.set_value("../invalid", window, cx));
    });
    draw(&mut surface);
    assert!(view.read_with(&surface, |form, _| form.validation.is_some()));
    view.update_in(&mut surface, |form, window, cx| {
        form.create_exclusions[2].update(cx, |state, cx| {
            state.set_value(" space;line\nbreak ", window, cx)
        });
    });
    draw(&mut surface);
    view.update(&mut surface, |form, cx| {
        let (requests, _) = form.capture(cx).expect("corrected rule");
        assert_eq!(
            requests[0]
                .create
                .as_ref()
                .expect("create")
                .exclude_patterns[2],
            " space;line\nbreak "
        );
    });
    draw(&mut surface);
    let viewport = surface
        .debug_bounds("archive-form-body")
        .expect("real form scroll owner");
    for _ in 0..60 {
        draw(&mut surface);
        if surface
            .debug_bounds("archive-create-rule-remove-2")
            .is_some_and(|bounds| {
                bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
            })
        {
            break;
        }
        let delta = surface
            .debug_bounds("archive-create-rule-remove-2")
            .map_or(-2., |bounds| {
                if bounds.top() < viewport.top() {
                    2.
                } else {
                    -2.
                }
            });
        surface.simulate_event(gpui_kit::ScrollEvent {
            position: gpui_kit::point(viewport.right() - px(12.), viewport.center().y),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., delta)),
            ..Default::default()
        });
    }
    let target = surface
        .debug_bounds("archive-create-rule-remove-2")
        .expect("remove target");
    assert!(
        target.top() >= viewport.top() && target.bottom() <= viewport.bottom(),
        "remove must be inside scroll viewport: target={target:?},viewport={viewport:?}"
    );
    click(&mut surface, "archive-create-rule-remove-2");
    assert_eq!(
        view.read_with(&surface, |form, _| form.create_exclusions.len()),
        2
    );
}

fn audit05_form_key(surface: &mut VisualTestContext, key: &'static str, down: bool, held: bool) {
    let keystroke = gpui_kit::Keystroke::parse(key).expect("form key");
    if down {
        surface.simulate_event(gpui_kit::KeyDownEvent {
            keystroke,
            is_held: held,
            prefer_character_input: false,
        });
    } else {
        surface.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    }
}
#[gpui_kit::test]
fn audit05_create_single_line_enter_reaches_original_submit_once(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        form.name.read(cx).focus_handle(cx).focus(window, cx)
    });
    draw(&mut surface);
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |form, _| form.submit_count),
        1,
        "actual focused single-line Enter must use original submit"
    );
    assert!(
        view.read_with(&surface, |form, _| form.error.is_some()),
        "original disconnected-owner admission failure remains observable"
    );
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |form, _| form.submit_count),
        1,
        "duplicate release is not another submit"
    );
}

#[gpui_kit::test]
fn audit05_form_ime_modifiers_repeat_and_busy_keep_original_validation(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        form.name.read(cx).focus_handle(cx).focus(window, cx);
    });
    draw(&mut surface);
    for key in ["ctrl-enter", "alt-enter", "shift-enter"] {
        audit05_form_key(&mut surface, key, true, false);
        audit05_form_key(&mut surface, key, false, false);
    }
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("character Enter"),
        is_held: false,
        prefer_character_input: true,
    });
    audit05_form_key(&mut surface, "enter", false, false);
    audit05_form_key(&mut surface, "enter", true, true);
    audit05_form_key(&mut surface, "enter", false, false);
    view.update_in(&mut surface, |form, window, cx| {
        form.name.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "中", Some(1..1), window, cx)
        });
    });
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);
    view.update_in(&mut surface, |form, window, cx| {
        form.name.update(cx, |input, cx| {
            input.unmark_text(window, cx);
            input.set_value("valid.zip", window, cx);
        });
        form.pending_picker = Some(PendingPicker {
            operation: None,
            purpose: ArchivePickerPurpose::Files,
        });
    });
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);
    view.update_in(&mut surface, |form, _, _| form.pending_picker = None);
    audit05_form_key(&mut surface, "enter", true, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 1);
    // Synchronous admission rejection clears busy; the same physical press still cannot retry.
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", true, true);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 1);
    audit05_form_key(&mut surface, "enter", false, false);
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 2);
}
#[gpui_kit::test]
fn audit05_form_masked_multiline_select_and_format_dialog_do_not_submit_outer(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    for password in [true, false] {
        view.update_in(&mut surface, |form, window, cx| {
            let input = if password {
                form.password.clone()
            } else {
                form.confirm.clone()
            };
            input.focus_handle(cx).focus(window, cx);
        });
        draw(&mut surface);
        audit05_form_key(&mut surface, "enter", true, false);
        audit05_form_key(&mut surface, "enter", false, false);
    }
    view.update_in(&mut surface, |form, window, cx| {
        form.exclusions.focus_handle(cx).focus(window, cx);
    });
    // Mount a real comment editor using the original production modify constructor.
    view.update_in(&mut surface, |form, window, cx| {
        *form = OperationForm::modify(
            ArchiveSettings::default(),
            audit05_modify_draft(ArchiveJobModifyAction::Comment),
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        form.exclusions.focus_handle(cx).focus(window, cx);
    });
    draw(&mut surface);
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);
    assert!(view.read_with(&surface, |form, cx| {
        form.exclusions.read(cx).value().contains('\n')
    }));
    view.update_in(&mut surface, |form, window, cx| {
        *form = OperationForm::create(
            ArchiveSettings::default(),
            vec![source()],
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        form.format.focus_handle(cx).focus(window, cx);
    });
    draw(&mut surface);
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |form, _| form.submit_count),
        0,
        "Select confirmation cannot fall through to outer submit"
    );
    assert!(
        surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|node| node.role() == Some(gpui_kit::Role::ComboBox))
            .all(|node| node.expanded() != Some(true))),
        "Select confirmation closes only its own layer"
    );
    view.update_in(&mut surface, |form, window, cx| {
        form.password.update(cx, |input, cx| {
            input.set_value("synthetic-private", window, cx)
        });
        form.confirm.update(cx, |input, cx| {
            input.set_value("synthetic-private", window, cx)
        });
        form.change_format(ArchiveCreateFormat::Tar, window, cx);
    });
    draw(&mut surface);
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |form, _| form.submit_count),
        0,
        "child format decision cannot submit outer form"
    );
}
fn audit05_modify_draft(action: ArchiveJobModifyAction) -> ModifyDraft {
    use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
    ModifyDraft {
        source: r"C:\synthetic\fixture.zip".into(),
        browser: ArchiveBrowserContext {
            snapshot_id: Uuid::new_v4(),
            format: "Zip".into(),
            comment: String::new(),
            sevenz_edit: None,
            tar_edit: None,
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [1; 32],
                size: 1,
                modified_unix_ns: 1,
            },
        },
        action,
        ordinals: vec![1],
        value: Some("new.txt".into()),
        source_prefix: None,
        added: vec![],
        codepage: 0,
    }
}
#[gpui_kit::test]
fn audit05_extract_and_edit_single_line_enter_keep_original_draft_and_cancel_key(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        *form = OperationForm::extract(
            ArchiveSettings::default(),
            r"C:\synthetic\fixture.zip".into(),
            None,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        form.destination.focus_handle(cx).focus(window, cx);
    });
    draw(&mut surface);
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 1);
    view.update_in(&mut surface, |form, window, cx| {
        *form = OperationForm::modify(
            ArchiveSettings::default(),
            audit05_modify_draft(ArchiveJobModifyAction::Rename),
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        form.name.focus_handle(cx).focus(window, cx);
    });
    draw(&mut surface);
    let original = view.read_with(&surface, |form, _| form.name.entity_id());
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 1);
    assert_eq!(
        view.read_with(&surface, |form, _| form.name.entity_id()),
        original
    );
    assert_eq!(
        view.read_with(&surface, |form, cx| form.name.read(cx).value().to_string()),
        "new.txt"
    );
    for _ in 0..30 {
        let focused = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .any(|node| node.label() == Some("取消") && node.focused() == Some(true))
        });
        if focused {
            break;
        }
        surface.simulate_keystrokes("tab");
        draw(&mut surface);
    }
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", false, false);
    surface.run_until_parked();
    assert!(view.read_with(&surface, |form, _| form.retired));
    assert_eq!(
        view.read_with(&surface, |form, _| form.submit_count),
        1,
        "focused Cancel is not default submit"
    );
}

#[gpui_kit::test]
fn extract_custom_destination_is_opt_in_and_cancel_retains_settings(cx: &mut TestAppContext) {
    let settings = ArchiveSettings {
        extract_custom_destination: Some(r"C:\synthetic\saved".into()),
        ..Default::default()
    };
    let (view, mut surface) = mount(cx, settings.clone());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                settings.clone(),
                r"C:\synthetic\archive.zip".into(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            assert_eq!(
                form.destination.read(cx).value().as_ref(),
                r"C:\synthetic\saved"
            );
            form.destination.update(cx, |input, cx| {
                input.set_value(r"C:\synthetic\changed", window, cx)
            });
            form.flag(Flag::RememberDestination, true, cx);
            form.remember = true;
            let (_, completion) = form
                .capture(cx)
                .expect("explicit validated submit snapshot");
            assert_eq!(
                completion
                    .remember
                    .expect("remember")
                    .extract_custom_destination
                    .as_deref(),
                Some(r"C:\synthetic\changed")
            );
            assert_eq!(
                form.settings, settings,
                "capturing/revalidating never mutates saved defaults"
            );
            form.retire(window, cx);
            assert_eq!(form.settings, settings, "Cancel never saves a draft");
        })
    });
}

#[gpui_kit::test]
fn extract_minimum_has_one_caption_and_reachable_path_browse_footer(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                Default::default(),
                r"C:\synthetic\archive.zip".into(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
        })
    });
    surface.simulate_resize(size(px(480.), px(360.)));
    draw(&mut surface);
    assert_eq!(
        surface
            .debug_bounds("archive-form-titlebar")
            .expect("one shared caption")
            .size
            .height,
        px(34.)
    );
    let viewport = surface
        .debug_bounds("archive-form-body")
        .expect("scroll owner");
    for _ in 0..50 {
        let target = surface
            .debug_bounds("archive-destination-picker")
            .expect("original browse Button");
        if target.top() >= viewport.top() && target.bottom() <= viewport.bottom() {
            break;
        }
        surface.simulate_event(gpui_kit::ScrollEvent {
            position: viewport.center(),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., -3.)),
            ..Default::default()
        });
        draw(&mut surface);
    }
    let target = surface
        .debug_bounds("archive-destination-picker")
        .expect("reachable browse");
    assert!(target.top() >= viewport.top() && target.bottom() <= viewport.bottom());
    for selector in ["archive-form-submit", "archive-form-cancel"] {
        let button = surface.debug_bounds(selector).expect("nonshrinking footer");
        assert!(button.top() >= viewport.bottom() && button.bottom() <= px(360.));
    }
}

#[gpui_kit::test]
fn here_source_picker_updates_locked_destination_without_changing_saved_preferences(
    cx: &mut TestAppContext,
) {
    let settings = ArchiveSettings {
        default_destination: ArchiveDestination::Here,
        extract_custom_destination: Some(r"C:\synthetic\saved".into()),
        ..Default::default()
    };
    let (view, mut surface) = mount(cx, settings.clone());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                settings.clone(),
                PathBuf::new(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            let operation = Uuid::new_v4();
            form.pending_picker = Some(PendingPicker {
                operation: Some(operation),
                purpose: ArchivePickerPurpose::Files,
            });
            form.picker_result(
                ArchivePickerResult {
                    version: 1,
                    operation_id: operation,
                    session_id: form.id,
                    generation: form.generation,
                    paths: vec![r"C:\synthetic\different\next.zip".into()],
                    cancelled: false,
                    code: None,
                    native_error_code: None,
                },
                window,
                cx,
            );
            let (requests, completion) = form.capture(cx).expect("new source Here request");
            assert_eq!(
                requests[0].destination.as_deref(),
                Some(r"C:\synthetic\different")
            );
            assert!(completion.remember.is_none());
            assert_eq!(form.settings, settings);
        })
    });
}

#[gpui_kit::test]
fn hz1_extract_form_has_actual_capacity_section_and_refresh(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                Default::default(),
                r"C:\synthetic\archive.zip".into(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
        })
    });
    surface.simulate_resize(size(px(560.), px(600.)));
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-space-preflight").is_some(),
        "extract must render the actual HZ1 capacity section before Start"
    );
    assert!(
        surface.debug_bounds("archive-space-refresh").is_some(),
        "original extract form must expose explicit capacity refresh"
    );
}

fn hz1_mount_extract(cx: &mut TestAppContext) -> (Entity<OperationForm>, VisualTestContext) {
    let (view, mut surface) = mount(cx, Default::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                Default::default(),
                r"C:\synthetic\archive.zip".into(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    surface.run_until_parked();
    (view, surface)
}
fn hz1_observed_plan() -> crate::archive::space::Plan {
    crate::archive::space::Plan {
        observation: nxtools_protocol::archive_space::ArchiveSpaceObservation {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            available_bytes: 8192,
            allocation_bytes: 4096,
            target_identity: "v1:target".into(),
            volume_identity: "v1:volume".into(),
            destination_missing: false,
            missing_destination_directories: Some(0),
            source_matches: true,
            source_set: None,
        },
        estimate: crate::archive::space::Estimate {
            payload: Some(1),
            allocation: Some(4096),
            lower_bound: 1,
            limitation: None,
        },
        concurrent_jobs: 0,
    }
}
#[gpui_kit::test]
fn hz1_actual_form_discards_changed_destination_strategy_scope_and_retired_result(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = hz1_mount_extract(cx);
    for change in 0..4 {
        surface.update(|window, cx| {
            view.update(cx, |form, cx| {
                form.space.pending = true;
                let epoch = form.space.epoch;
                match change {
                    0 => form.destination.update(cx, |input, cx| {
                        input.set_value(r"C:\synthetic\new-target", window, cx)
                    }),
                    1 => {
                        if let Draft::Extract(draft) = &mut form.draft {
                            draft.strategy = ArchiveDestination::Name;
                        }
                    }
                    2 => {
                        if let Draft::Extract(draft) = &mut form.draft {
                            draft.source = r"C:\synthetic\replacement.zip".into();
                        }
                    }
                    _ => form.retire(window, cx),
                }
                form.apply_space(epoch, Ok(hz1_observed_plan()), cx);
                assert!(
                    form.space.plan.is_none(),
                    "late observation cannot replace changed/retired draft"
                );
                assert!(!form.space.pending);
            })
        });
    }
}
#[gpui_kit::test]
fn hz1_actual_form_errors_and_unknown_do_not_block_original_start(cx: &mut TestAppContext) {
    let (view, mut surface) = hz1_mount_extract(cx);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.space.pending = true;
            let epoch = form.space.epoch;
            form.apply_space(epoch, Err("权限不足（Windows 5）".into()), cx);
            assert!(!form.space.pending && form.space.plan.is_none());
            assert!(
                form.space
                    .error
                    .as_deref()
                    .is_some_and(|message| message.contains("权限不足"))
            );
            let (requests, _) = form
                .capture(cx)
                .expect("preflight failure does not introduce hard block");
            assert_eq!(requests[0].limits, ArchiveJobLimits::default());
            form.space.pending = true;
            let mut plan = hz1_observed_plan();
            plan.estimate = Default::default();
            form.apply_space(epoch, Ok(plan), cx);
            assert!(
                form.space
                    .plan
                    .as_ref()
                    .expect("unknown observation")
                    .estimate
                    .allocation
                    .is_none()
            );
            assert!(form.capture(cx).is_ok());
        })
    });
    surface.simulate_resize(size(px(480.), px(360.)));
    draw(&mut surface);
    let viewport = surface
        .debug_bounds("archive-form-body")
        .expect("original scroll owner");
    for selector in ["archive-form-submit", "archive-form-cancel"] {
        let button = surface.debug_bounds(selector).expect("original footer");
        assert!(button.top() >= viewport.bottom() && button.bottom() <= px(360.));
    }
}

#[gpui_kit::test]
fn hz1_output_options_and_source_retirement_invalidate_displayed_plan(cx: &mut TestAppContext) {
    let (view, mut surface) = hz1_mount_extract(cx);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            for change in 0..2 {
                form.space.plan = Some(hz1_observed_plan());
                if let Draft::Extract(draft) = &mut form.draft {
                    if change == 0 {
                        draft.copy_motw = !draft.copy_motw;
                    } else {
                        draft.single_file_folder = !draft.single_file_folder;
                        draft.strategy = ArchiveDestination::Smart;
                    }
                }
                form.revalidate(cx);
                assert!(
                    form.space.plan.is_none(),
                    "output-option change invalidates plan"
                );
            }
            form.space.plan = Some(hz1_observed_plan());
            form.invalidate_space_source(std::path::Path::new(r"C:\synthetic\archive.zip"), cx);
            assert!(form.space.plan.is_none() && !form.space.pending);
            assert!(
                form.space
                    .error
                    .as_ref()
                    .is_some_and(|message| message.contains("压缩包已变化"))
            );
        })
    });
}

#[gpui_kit::test]
fn sevenz_edit_original_form_requires_one_regular_replacement_and_discloses_reencode(
    cx: &mut TestAppContext,
) {
    let root = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("sevenz-replacement-form-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).expect("owned root");
    let replacement = root.join("replacement.txt");
    std::fs::write(&replacement, b"replacement").expect("owned replacement");
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        let mut draft = audit05_modify_draft(ArchiveJobModifyAction::ReplaceContent);
        draft.browser.format = "SevenZ".into();
        draft.browser.sevenz_edit =
            Some(nxtools_protocol::archive_browser::SevenzEditEligibility { refusal: None });
        draft.value = None;
        *form = OperationForm::modify(
            ArchiveSettings::default(),
            draft,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        assert!(form.capture(cx).is_err(), "replacement not chosen");
    });
    draw(&mut surface);
    let warning = surface
        .debug_bounds("archive-sevenz-cost")
        .expect("actual important reencode disclosure painted");
    assert!(
        warning.size.height > px(0.) && warning.top() >= px(33.) && warning.bottom() < px(512.)
    );
    assert!(
        SEVENZ_EDIT_COST.contains("完整解码并重编码为非固实 LZMA2")
            && SEVENZ_EDIT_COST.contains("并存空间")
            && SEVENZ_EDIT_COST.contains("未知")
    );
    assert!(surface.update(|window, _| {
        !gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("添加文件夹…"))
    }));
    view.update_in(&mut surface, |form, window, cx| {
        let operation = Uuid::new_v4();
        form.pending_picker = Some(PendingPicker {
            operation: Some(operation),
            purpose: ArchivePickerPurpose::Files,
        });
        form.picker_result(
            ArchivePickerResult {
                version: 1,
                operation_id: operation,
                session_id: form.id,
                generation: form.generation,
                paths: vec![root.to_string_lossy().into_owned()],
                cancelled: false,
                code: None,
                native_error_code: None,
            },
            window,
            cx,
        );
    });
    draw(&mut surface);
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft, Draft::Modify(draft) if draft.added.is_empty()))
    });
    view.update_in(&mut surface, |form, window, cx| {
        let operation = Uuid::new_v4();
        form.pending_picker = Some(PendingPicker {
            operation: Some(operation),
            purpose: ArchivePickerPurpose::Files,
        });
        let mut result = ArchivePickerResult {
            version: 1,
            operation_id: operation,
            session_id: form.id,
            generation: form.generation - 1,
            paths: vec![replacement.to_string_lossy().into_owned()],
            cancelled: false,
            code: None,
            native_error_code: None,
        };
        form.picker_result(result.clone(), window, cx);
        assert!(
            matches!(&form.draft, Draft::Modify(draft) if draft.added.is_empty()),
            "old picker result stays retired"
        );
        result.generation = form.generation;
        form.picker_result(result, window, cx);
    });
    draw(&mut surface);
    view.update_in(&mut surface, |form, window, cx| {
        let (requests, completion) = form.capture(cx).expect("one ordinary file");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].paths.len(), 2);
        let modify = requests[0].modify.as_ref().expect("original Modify intent");
        assert_eq!(modify.action, ArchiveJobModifyAction::ReplaceContent);
        assert_eq!(modify.ordinals, vec![1]);
        assert!(modify.value.is_none() && completion.remember.is_none());
        let name_entity = form.name.entity_id();
        form.submit(window, cx);
        form.submit(window, cx);
        assert_eq!(
            form.submit_count, 1,
            "original form pending gate prevents duplicate dispatch"
        );
        assert!(form.submitted);
        form.fail_submit("native rejected".into(), window, cx);
        assert_eq!(name_entity, form.name.entity_id());
        assert!(form.capture(cx).is_ok(), "original draft remains retryable");
        if let Draft::Modify(draft) = &mut form.draft {
            draft.added.push(replacement.to_string_lossy().into_owned());
            assert!(
                draft.request().is_err(),
                "not disguised Add/multiple replacements"
            );
            draft.added.pop();
            draft.browser.sevenz_edit = None;
            assert!(draft.request().is_err(), "legacy unproven facts refuse");
            draft.browser.format = "Zip".into();
            assert!(
                draft.request().is_err(),
                "ZIP replacement stays unavailable"
            );
        }
    });
    std::fs::remove_file(replacement).expect("owned replacement cleanup");
    std::fs::remove_dir(root).expect("owned root cleanup");
}

#[gpui_kit::test]
fn sevenz_implicit_directory_form_keeps_original_prefix_while_target_changes(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        let mut draft = audit05_modify_draft(ArchiveJobModifyAction::Rename);
        draft.browser.format = "SevenZ".into();
        draft.browser.sevenz_edit =
            Some(nxtools_protocol::archive_browser::SevenzEditEligibility { refusal: None });
        draft.ordinals = vec![71];
        draft.value = Some("a/".into());
        draft.source_prefix = Some("a/".into());
        *form = OperationForm::modify(
            ArchiveSettings::default(),
            draft,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
        form.name
            .update(cx, |name, cx| name.set_value("renamed/", window, cx));
        let request = form.capture(cx).expect("valid subtree rename").0.remove(0);
        let modify = request.modify.expect("original typed intent");
        assert_eq!(modify.ordinals, vec![71]);
        assert_eq!(modify.value.as_deref(), Some("renamed/"));
        assert_eq!(
            modify.source_prefix.as_deref(),
            Some("a/"),
            "not inferred a/b/"
        );
    });
}

#[gpui_kit::test]
fn tar_edit_original_form_discloses_cost_and_keeps_one_replacement_and_prefix(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    for format in ["Tar", "TarGz", "TarXz"] {
        view.update_in(&mut surface, |form, window, cx| {
            let mut draft = audit05_modify_draft(ArchiveJobModifyAction::Rename);
            draft.browser.format = format.into();
            draft.browser.tar_edit =
                Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
            draft.ordinals = vec![71];
            draft.value = Some("a/".into());
            draft.source_prefix = Some("a/".into());
            *form = OperationForm::modify(
                ArchiveSettings::default(),
                draft,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            form.name
                .update(cx, |name, cx| name.set_value("renamed/", window, cx));
            let request = form.capture(cx).expect("TAR original form").0.remove(0);
            let edit = request.modify.expect("Modify");
            assert_eq!(edit.ordinals, vec![71]);
            assert_eq!(edit.source_prefix.as_deref(), Some("a/"));
            assert_eq!(edit.value.as_deref(), Some("renamed/"));
        });
        draw(&mut surface);
        let warning = surface
            .debug_bounds("archive-tar-cost")
            .expect("real official Alert cost");
        assert!(warning.size.height > px(0.) && warning.top() >= px(33.));
        view.update_in(&mut surface, |form, _, cx| {
            if let Draft::Modify(draft) = &mut form.draft {
                draft.browser.tar_edit = None;
            }
            assert!(
                form.capture(cx).is_err(),
                "missing completed worker facts fail closed"
            );
        });
    }
    assert!(
        TAR_EDIT_COST.contains("完整重写")
            && TAR_EDIT_COST.contains("并存空间")
            && TAR_EDIT_COST.contains("估算未知")
            && TAR_EDIT_COST.contains("没有内容校验和")
    );
}

#[gpui_kit::test]
fn step4_add_form_displays_internal_folder_and_truthful_conflict_policy(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        let mut draft = audit05_modify_draft(ArchiveJobModifyAction::Add);
        draft.ordinals.clear();
        draft.value = Some("资料/目标/".into());
        *form = OperationForm::modify(
            ArchiveSettings::default(),
            draft,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
    });
    draw(&mut surface);
    surface.update(|window, _| {
        let labels = gpui_kit::base::test_support::snapshots(window);
        assert!(
            labels.iter().any(|node| node.label() == Some("资料/目标/")),
            "actual internal destination missing"
        );
        assert!(
            labels
                .iter()
                .any(|node| node.label().is_some_and(|label| label.contains("逐项询问"))),
            "actual Add policy must be visible"
        );
    });
}

#[gpui_kit::test]
fn hz5_reclamation_requires_unchecked_control_and_separate_destructive_acknowledgement(
    cx: &mut TestAppContext,
) {
    let settings = ArchiveSettings {
        default_destination: ArchiveDestination::Here,
        conflict_policy: ArchiveConflictPolicy::Ask,
        ..Default::default()
    };
    let (view, mut surface) = mount(cx, settings.clone());
    view.update_in(&mut surface, |form, window, cx| {
        *form = OperationForm::extract(
            settings,
            r"C:\synthetic\stored.zip".into(),
            None,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
    });
    surface.simulate_resize(size(px(680.), px(1000.)));
    draw(&mut surface);
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft, Draft::Extract(draft) if !draft.reclamation));
    });
    assert!(surface.debug_bounds("archive-reclamation").is_some());
    assert!(
        surface
            .debug_bounds("archive-reclamation-warning")
            .is_none(),
        "irreversible warning stays absent before explicit opt-in"
    );

    click(&mut surface, "archive-reclamation");
    assert!(
        surface
            .debug_bounds("archive-reclamation-warning")
            .is_some(),
        "irreversible warning appears after explicit opt-in"
    );
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft, Draft::Extract(draft) if draft.reclamation));
    });
    click(&mut surface, "archive-form-submit");
    view.read_with(&surface, |form, _| {
        assert!(form.reclamation_confirmation_pending);
        assert!(!form.submitted);
        assert_eq!(form.submit_count, 0, "no admission before acknowledgement");
    });
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    click(&mut surface, "archive-reclamation-confirm");
    surface.run_until_parked();
    view.read_with(&surface, |form, _| {
        assert!(!form.reclamation_confirmation_pending);
        assert_eq!(form.submit_count, 1, "explicit acknowledgement admits once");
    });
}

#[gpui_kit::test]
fn workflow_empty_source_has_no_capacity_table_or_reclamation_warning(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, Default::default());
    view.update_in(&mut surface, |form, window, cx| {
        *form = OperationForm::extract(
            Default::default(),
            PathBuf::new(),
            None,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
    });
    surface.simulate_resize(size(px(560.), px(600.)));
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-space-values").is_none());
    assert!(
        surface
            .debug_bounds("archive-reclamation-warning")
            .is_none(),
        "ordinary initial state has no destructive warning"
    );
    let section = surface
        .debug_bounds("archive-space-preflight")
        .expect("space guidance");
    assert!(
        section.size.height < px(80.),
        "empty space state must not render an empty capacity report: {section:?}"
    );
}

#[gpui_kit::test]
fn workflow_destination_precedes_advisory_and_footer_stays_reachable(cx: &mut TestAppContext) {
    let (_, mut surface) = hz1_mount_extract(cx);
    surface.simulate_resize(size(px(560.), px(600.)));
    draw(&mut surface);
    let destination = surface
        .debug_bounds("archive-destination-group")
        .expect("actual target control");
    let space = surface
        .debug_bounds("archive-space-preflight")
        .expect("actual advisory");
    assert!(
        destination.bottom() <= space.top(),
        "decide target before its capacity: {destination:?} {space:?}"
    );
    let footer = surface
        .debug_bounds("archive-form-submit")
        .expect("primary action");
    assert!(footer.bottom() <= px(600.) && footer.size.height == px(32.));
}

fn mount_continuation(
    cx: &mut TestAppContext,
    mode: &'static str,
) -> (Entity<OperationForm>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let capture = Rc::new(RefCell::new(None));
    let slot = capture.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut form = OperationForm::continue_reclamation(
                ArchiveSettings::default(),
                r"C:\synthetic\very-long\source\original-stored-archive.zip".into(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            form.preview_recovery_state(mode, window, cx);
            form
        });
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(560.), px(600.)));
    draw(&mut surface);
    (
        capture.borrow().clone().expect("continuation form"),
        surface,
    )
}

fn recovery_descriptor(
    form: &OperationForm,
    recovery_id: Uuid,
    destination: Option<&str>,
    copy_motw: bool,
    completed: bool,
) -> nxtools_protocol::archive_recovery::ArchiveRecoveryInspection {
    nxtools_protocol::archive_recovery::ArchiveRecoveryInspection {
        version: 1,
        operation_id: Uuid::new_v4(),
        session_id: form.id,
        generation: form.generation,
        recovery_id,
        source_identity: "v1:1:00112233445566778899AABBCCDDEEFF:4096".into(),
        destination: destination.map(str::to_owned),
        copy_motw,
        completed,
    }
}

fn form_button_disabled(surface: &mut VisualTestContext, label: &str) -> Option<bool> {
    surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.role() == Some(gpui_kit::Role::Button) && node.label() == Some(label))
            .and_then(|node| node.disabled())
    })
}

#[gpui_kit::test]
fn continuation_production_form_has_only_source_bound_recovery_controls(cx: &mut TestAppContext) {
    let (_, mut surface) = mount_continuation(cx, "continue-found");
    assert!(
        surface
            .debug_bounds("archive-recovery-source-group")
            .is_some()
    );
    assert!(
        surface
            .debug_bounds("archive-recovery-destination-group")
            .is_some()
    );
    for selector in [
        "archive-extract-source",
        "archive-space-preflight",
        "archive-options-toggle",
        "archive-remember",
    ] {
        assert!(
            surface.debug_bounds(selector).is_none(),
            "continuation mounted forbidden control {selector}"
        );
    }
    surface.update(|window, _| {
        let labels = gpui_kit::base::test_support::snapshots(window);
        for forbidden in [
            "恢复标识",
            "文件范围",
            "文件夹安排",
            "同名文件",
            "磁盘空间",
            "记住本次设置",
            "选择已标记压缩包…",
        ] {
            assert!(
                !labels.iter().any(|node| node.label() == Some(forbidden)),
                "continuation exposed {forbidden}"
            );
        }
    });
    assert_eq!(
        surface
            .debug_bounds("archive-form-submit")
            .expect("Medium continuation Start")
            .size
            .height,
        px(32.)
    );
}

#[gpui_kit::test]
fn continuation_pending_error_completed_and_missing_target_block_start_and_enter(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount_continuation(cx, "continue-pending");
    for (index, mode) in [
        "continue-pending",
        "continue-error",
        "continue-completed",
        "continue-missing",
    ]
    .into_iter()
    .enumerate()
    {
        if index != 0 {
            view.update_in(&mut surface, |form, window, cx| {
                form.preview_recovery_state(mode, window, cx)
            });
            draw(&mut surface);
        }
        assert_eq!(
            form_button_disabled(&mut surface, "继续空间回收解压"),
            Some(true),
            "{mode} Start must be disabled"
        );
        assert_ne!(
            form_button_disabled(&mut surface, "取消"),
            Some(true),
            "{mode} Cancel remains reachable"
        );
        view.update_in(&mut surface, |form, window, cx| {
            form.destination.focus_handle(cx).focus(window, cx);
        });
        audit05_form_key(&mut surface, "enter", true, false);
        audit05_form_key(&mut surface, "enter", false, false);
        draw(&mut surface);
        view.read_with(&surface, |form, _| {
            assert_eq!(form.submit_count, 0, "{mode} Enter cannot submit");
            assert!(!form.reclamation_confirmation_pending);
        });
        if mode == "continue-error" {
            assert!(
                surface
                    .debug_bounds("archive-recovery-destination-picker")
                    .is_some(),
                "failed target inspection must allow choosing the original folder again"
            );
            assert_eq!(
                surface
                    .debug_bounds("archive-recovery-retry")
                    .expect("Small retry")
                    .size
                    .height,
                px(24.)
            );
        }
        if mode == "continue-completed" {
            assert!(surface.debug_bounds("archive-recovery-warning").is_none());
            assert!(surface.debug_bounds("archive-open-after").is_none());
            assert!(surface.debug_bounds("archive-keep-open").is_none());
        }
        if mode == "continue-missing" {
            assert_eq!(
                surface
                    .debug_bounds("archive-recovery-destination-picker")
                    .expect("existing Medium folder picker")
                    .size
                    .height,
                px(32.)
            );
        }
    }
}

#[gpui_kit::test]
fn continuation_inspection_restores_internal_facts_without_autosubmit(cx: &mut TestAppContext) {
    let (view, mut surface) = mount_continuation(cx, "continue-pending");
    let original_destination = r"C:\synthetic\original-output";
    let recovery_id = Uuid::new_v4();
    view.update_in(&mut surface, |form, window, cx| {
        form.begin_recovery(Some(original_destination.into()), window, cx);
        let epoch = form.recovery.epoch;
        let descriptor =
            recovery_descriptor(form, recovery_id, Some(original_destination), true, false);
        form.apply_recovery(epoch, Ok(descriptor), window, cx);
        assert_eq!(form.submit_count, 0);
        assert!(!form.reclamation_confirmation_pending && !form.submitted);
        form.destination.update(cx, |input, cx| {
            input.set_value(r"C:\synthetic\tampered-display", window, cx)
        });
        form.recovery_id.update(cx, |input, cx| {
            input.set_value(Uuid::nil().to_string(), window, cx)
        });
        let (requests, completion) = form.capture(cx).expect("fresh Continue request");
        let request = &requests[0];
        let extract = request.extract.as_ref().expect("extract request");
        let intent = extract.reclamation.expect("internal continuation intent");
        assert_eq!(
            request.paths,
            vec![r"C:\synthetic\very-long\source\original-stored-archive.zip"]
        );
        assert_eq!(request.destination.as_deref(), Some(original_destination));
        assert_eq!(intent.mode, ArchiveReclamationMode::Continue);
        assert_eq!(intent.recovery_id, recovery_id);
        assert!(extract.copy_motw);
        assert!(completion.remember.is_none() && completion.remember_baseline.is_none());
        assert!(form.space.key.is_none() && !form.space.pending);
    });
    draw(&mut surface);
    view.read_with(&surface, |form, _| {
        assert_eq!(form.submit_count, 0, "inspection never auto-submits")
    });
    click(&mut surface, "archive-form-submit");
    view.read_with(&surface, |form, _| {
        assert!(form.reclamation_confirmation_pending);
        assert_eq!(form.submit_count, 0, "explicit warning precedes admission");
    });
}

#[gpui_kit::test]
fn continuation_picker_reinspects_candidate_and_ignores_stale_reply(cx: &mut TestAppContext) {
    let (view, mut surface) = mount_continuation(cx, "continue-missing");
    let candidate = r"C:\synthetic\original-output";
    let fresh_recovery_id = Uuid::new_v4();
    view.update_in(&mut surface, |form, window, cx| {
        let stale_epoch = form.recovery.epoch;
        let operation = Uuid::new_v4();
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
                paths: vec![candidate.into()],
                cancelled: false,
                code: None,
                native_error_code: None,
            },
            window,
            cx,
        );
        let fresh_epoch = form.recovery.epoch;
        assert!(fresh_epoch > stale_epoch && form.recovery.pending);
        assert!(form.recovery.inspection.is_none());
        assert!(form.recovery_request_current(
            fresh_epoch,
            r"C:\synthetic\very-long\source\original-stored-archive.zip",
            Some(candidate),
            cx,
        ));
        let stale = recovery_descriptor(form, Uuid::new_v4(), Some(candidate), false, false);
        form.apply_recovery(stale_epoch, Ok(stale), window, cx);
        assert!(form.recovery.pending && form.capture(cx).is_err());
        let fresh = recovery_descriptor(form, fresh_recovery_id, Some(candidate), false, false);
        form.apply_recovery(fresh_epoch, Ok(fresh), window, cx);
        let request = form
            .capture(cx)
            .expect("fresh candidate inspection")
            .0
            .remove(0);
        assert_eq!(request.destination.as_deref(), Some(candidate));
        assert_eq!(
            request
                .extract
                .expect("extract")
                .reclamation
                .expect("Continue")
                .recovery_id,
            fresh_recovery_id
        );
        assert!(!form.recovery_request_current(
            stale_epoch,
            r"C:\synthetic\very-long\source\original-stored-archive.zip",
            None,
            cx,
        ));
    });
}

#[gpui_kit::test]
fn extract_source_is_bound_only_for_contextual_form(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let selection = DraftSelection {
        browser: audit05_modify_draft(ArchiveJobModifyAction::Add).browser,
        ordinals: vec![0],
    };
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                ArchiveSettings::default(),
                r"C:\synthetic\context.zip".into(),
                Some(selection),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
        });
    });
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            let operation = Uuid::new_v4();
            form.pending_picker = Some(PendingPicker { operation: Some(operation), purpose: ArchivePickerPurpose::Files });
            form.picker_result(ArchivePickerResult {
                version: 1, operation_id: operation, session_id: form.id,
                generation: form.generation, paths: vec![r"C:\synthetic\replacement.zip".into()],
                cancelled: false, code: None, native_error_code: None,
            }, window, cx);
            assert!(matches!(&form.draft, Draft::Extract(d) if d.source.as_path() == std::path::Path::new(r"C:\synthetic\context.zip") && d.selection.is_some()));
        });
    });
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-extract-source").is_none());
    view.read_with(cx, |form, _| {
        assert!(form.source_bound);
        assert!(matches!(&form.draft, Draft::Extract(d) if d.selection.is_some()));
    });
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            *form = OperationForm::extract(
                ArchiveSettings::default(),
                PathBuf::new(),
                None,
                WeakEntity::new_invalid(),
                window,
                cx,
            );
        });
    });
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-extract-source").is_some());
    view.read_with(cx, |form, _| assert!(!form.source_bound));
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            let operation = Uuid::new_v4();
            form.pending_picker = Some(PendingPicker { operation: Some(operation), purpose: ArchivePickerPurpose::Files });
            form.picker_result(ArchivePickerResult {
                version: 1, operation_id: operation, session_id: form.id,
                generation: form.generation, paths: vec![r"C:\synthetic\chosen.zip".into()],
                cancelled: false, code: None, native_error_code: None,
            }, window, cx);
            assert!(matches!(&form.draft, Draft::Extract(d) if d.source.as_path() == std::path::Path::new(r"C:\synthetic\chosen.zip")));
        });
    });
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-extract-source").is_some());
}

#[gpui_kit::test]
fn picker_receipt_failure_and_disconnect_release_busy_form(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let operation = Uuid::new_v4();
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.pending_picker = Some(PendingPicker {
                operation: Some(operation),
                purpose: ArchivePickerPurpose::Destination,
            });
            form.picker_receipt_failure(operation, "选择器请求被拒绝，请重试。", cx);
            assert!(!form.busy());
            assert!(form.error.is_some());
            form.pending_picker = Some(PendingPicker {
                operation: Some(operation),
                purpose: ArchivePickerPurpose::Destination,
            });
            form.picker_disconnected(cx);
            assert!(!form.busy());
            assert!(form.error.is_some());
        });
    });
}

#[gpui_kit::test]
fn opt03_add_remove_uses_real_rows_and_original_picker_lifecycle(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let sources: Vec<String> = ["a", "b", "c", "d"]
        .map(|name| format!(r"C:\synthetic\{name}.txt"))
        .into();
    view.update_in(&mut surface, |form, window, cx| {
        let mut draft = audit05_modify_draft(ArchiveJobModifyAction::Add);
        draft.ordinals.clear();
        draft.value = Some("folder/".into());
        draft.added = sources.clone();
        *form = OperationForm::modify(
            ArchiveSettings::default(),
            draft,
            WeakEntity::new_invalid(),
            window,
            cx,
        );
    });
    draw(&mut surface);
    let frozen = view.read_with(&surface, |form, _| match &form.draft {
        Draft::Modify(draft) => draft.browser.clone(),
        _ => panic!("Add form"),
    });
    click(&mut surface, "archive-edit-remove-1");
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft, Draft::Modify(draft) if draft.added == [sources[0].clone(), sources[2].clone(), sources[3].clone()]));
    });
    click(&mut surface, "archive-edit-remove-0");
    click(&mut surface, "archive-edit-remove-1");
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft, Draft::Modify(draft) if draft.added == [sources[2].clone()]));
    });
    let operation = Uuid::new_v4();
    view.update_in(&mut surface, |form, _, cx| {
        form.pending_picker = Some(PendingPicker {
            operation: Some(operation),
            purpose: ArchivePickerPurpose::Files,
        });
        cx.notify();
    });
    click(&mut surface, "archive-edit-remove-0");
    view.read_with(&surface, |form, _| {
        assert!(matches!(&form.draft, Draft::Modify(draft) if draft.added.len() == 1));
    });
    view.update_in(&mut surface, |form, window, cx| {
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
    });
    click(&mut surface, "archive-edit-remove-0");
    assert!(surface.debug_bounds("archive-edit-sources-empty").is_some());
    view.update_in(&mut surface, |form, _, cx| {
        assert!(form.capture(cx).is_err());
        assert!(form.validation.is_some());
        assert!(!form.submitted);
        let Draft::Modify(draft) = &form.draft else {
            panic!("Add form")
        };
        assert!(draft.added.is_empty());
        assert_eq!(draft.browser, frozen);
        assert_eq!(draft.value.as_deref(), Some("folder/"));
    });
    view.update_in(&mut surface, |form, window, cx| {
        let operation = Uuid::new_v4();
        form.pending_picker = Some(PendingPicker {
            operation: Some(operation),
            purpose: ArchivePickerPurpose::Files,
        });
        form.picker_result(
            ArchivePickerResult {
                version: 1,
                operation_id: operation,
                session_id: form.id,
                generation: form.generation,
                paths: vec![sources[1].clone()],
                cancelled: false,
                code: None,
                native_error_code: None,
            },
            window,
            cx,
        );
        let (requests, _) = form.capture(cx).expect("re-added source");
        assert_eq!(
            requests[0].paths,
            vec![r"C:\synthetic\fixture.zip".to_owned(), sources[1].clone()]
        );
        assert_eq!(
            requests[0].modify.as_ref().expect("Add").value.as_deref(),
            Some("folder/")
        );
    });
}

#[gpui_kit::test]
fn opt03_ctrl_v_preserves_text_input_focus(cx: &mut TestAppContext) {
    let (form, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
            "text-only-name.zip".into(),
        ));
        let input = form.read(cx).name.clone();
        input.focus_handle(cx).focus(window, cx);
    });
    surface.simulate_keystrokes("ctrl-a ctrl-v");
    draw(&mut surface);
    form.read_with(&surface, |form, cx| {
        assert_eq!(form.name.read(cx).value().as_ref(), "text-only-name.zip");
        assert!(
            form.pending_clipboard.is_none(),
            "text paste never requests file intake"
        );
        let Draft::Create(draft) = &form.draft else {
            panic!("Create draft")
        };
        assert_eq!(draft.sources.len(), 1);
        assert_eq!(draft.sources[0].path, source().path);
    });
}

#[gpui_kit::test]
fn archive_polish_sources_have_bounded_viewport_and_removal(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            if let Draft::Create(draft) = &mut form.draft {
                draft.sources = (0..32)
                    .map(|index| DraftSource {
                        path: format!(r"C:\synthetic\deep\source-{index}.txt").into(),
                        directory: false,
                        bytes: Some(12),
                    })
                    .collect();
            }
            form.revalidate(cx);
        })
    });
    draw(&mut surface);
    let sources = surface
        .debug_bounds("archive-create-source-viewport")
        .expect("independent source viewport");
    assert!(sources.size.height <= px(144.));
    let footer = surface
        .debug_bounds("archive-form-submit")
        .expect("fixed footer");
    assert!(sources.bottom() < footer.top());
    click(&mut surface, "archive-create-remove-source-0");
    view.read_with(&surface, |form, _| {
        let Draft::Create(draft) = &form.draft else {
            panic!("create draft")
        };
        assert_eq!(draft.sources.len(), 31);
        assert!(draft.sources[0].path.ends_with("source-1.txt"));
    });
}

#[gpui_kit::test]
fn ref17_format_change_preserves_split_input_until_explicit_none(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            form.volume
                .update(cx, |input, cx| input.set_value("128", window, cx));
            form.format
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("TAR"))));
        });
    });
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            assert_eq!(
                form.volume.read(cx).value().as_ref(),
                "128",
                "unsupported format must not erase the intended split size"
            );
            assert!(
                form.capture(cx).is_err(),
                "TAR with retained split must require an explicit decision"
            );
        });
    });
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.format
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("ZIP"))))
        });
    });
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            let (requests, _) = form.capture(cx).expect("restored ZIP split");
            assert_eq!(
                requests[0].create.as_ref().expect("create").volume_bytes,
                Some(134_217_728)
            );
        });
    });
}

#[gpui_kit::test]
fn ref17_presets_units_checked_bytes_and_reopen_use_original_form(cx: &mut TestAppContext) {
    use nxtools_core::archive::{ArchiveVolumeSize, ArchiveVolumeUnit};
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let identity = view.read_with(&surface, |form, _| form.volume.entity_id());
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.volume_preset
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("2 GiB"))));
        })
    });
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            let (requests, _) = form.capture(cx).expect("preset");
            assert_eq!(
                requests[0].create.as_ref().expect("create").volume_bytes,
                Some(2_147_483_648)
            );
            form.volume_unit
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("KiB"))));
        })
    });
    draw(&mut surface);
    surface.update(|window, cx| {
        view.update(cx, |form, cx| {
            assert_eq!(form.volume.read(cx).value().as_ref(), "2097152");
            assert_eq!(form.volume.entity_id(), identity);
            form.volume.update(cx, |input, cx| {
                input.set_value("1536", window, cx);
                cx.emit(InputEvent::Change);
            });
            form.volume_unit
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("MiB"))));
        })
    });
    draw(&mut surface);
    let remembered = surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            assert_eq!(
                form.volume.read(cx).value().as_ref(),
                "1536",
                "inexact unit choice must not round"
            );
            assert!(form.error.is_some());
            assert_eq!(form.volume_unit.read(cx).selected_value(), Some(&"KiB"));
            form.remember = true;
            let (requests, completion) = form.capture(cx).expect("retained exact custom");
            assert_eq!(
                requests[0].create.as_ref().expect("create").volume_bytes,
                Some(1_572_864)
            );
            let remembered = completion.remember.expect("explicit remember");
            assert_eq!(
                remembered.create_volume_size,
                Some(ArchiveVolumeSize {
                    value: 1536,
                    unit: ArchiveVolumeUnit::KiB
                })
            );
            remembered
        })
    });
    let (reopened, mut reopened_surface) = mount(cx, remembered);
    draw(&mut reopened_surface);
    reopened_surface.update(|_, cx| {
        reopened.update(cx, |form, cx| {
            assert_eq!(form.volume.read(cx).value().as_ref(), "1536");
            assert_eq!(form.volume_unit.read(cx).selected_value(), Some(&"KiB"));
            assert_eq!(
                form.capture(cx).expect("reopened exact size").0[0]
                    .create
                    .as_ref()
                    .expect("create")
                    .volume_bytes,
                Some(1_572_864)
            );
        })
    });
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.volume_preset
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("不分卷"))));
        })
    });
    draw(&mut surface);
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            assert_eq!(
                form.capture(cx).expect("explicit none").0[0]
                    .create
                    .as_ref()
                    .expect("create")
                    .volume_bytes,
                None
            );
        })
    });
}

#[gpui_kit::test]
fn ref17_positive_integer_limits_stay_at_production_submit(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    surface.update(|_, cx| {
        view.update(cx, |form, cx| {
            form.volume_unit
                .update(cx, |_, cx| cx.emit(SelectEvent::Confirm(Some("KiB"))));
        })
    });
    draw(&mut surface);
    for (value, expected) in [
        ("64", Some(65_536)),
        ("65", Some(66_560)),
        ("0", None),
        ("63", None),
        ("1.5", None),
        ("-1", None),
        ("18446744073709551615", None),
        ("18446744073709551616", None),
    ] {
        surface.update(|window, cx| {
            view.update(cx, |form, cx| {
                form.volume
                    .update(cx, |input, cx| input.set_value(value, window, cx));
                match expected {
                    Some(bytes) => assert_eq!(
                        form.capture(cx).expect("valid integer size").0[0]
                            .create
                            .as_ref()
                            .expect("create")
                            .volume_bytes,
                        Some(bytes)
                    ),
                    None => {
                        form.submit(window, cx);
                        assert!(!form.submitted);
                        assert!(form.error.is_some(), "invalid value {value}");
                    }
                }
            })
        });
    }
}

#[gpui_kit::test]
fn ref17_actual_number_input_unit_keyboard_and_fixed_footer(cx: &mut TestAppContext) {
    for dimensions in [size(px(680.), px(600.)), size(px(480.), px(360.))] {
        let (view, mut surface) = mount(cx, ArchiveSettings::default());
        surface.simulate_resize(dimensions);
        draw(&mut surface);
        let viewport = surface
            .debug_bounds("archive-form-body")
            .expect("original scroll body");
        for _ in 0..60 {
            let target = surface.debug_bounds("archive-options-toggle");
            if target.is_some_and(|bounds| {
                bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
            }) {
                break;
            }
            surface.simulate_event(gpui_kit::ScrollEvent {
                position: viewport.center(),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., -2.)),
                ..Default::default()
            });
            draw(&mut surface);
        }
        click(&mut surface, "archive-options-toggle");
        assert!(
            view.read_with(&surface, |form, _| form.space.options_open),
            "actual disclosure click"
        );
        for _ in 0..60 {
            draw(&mut surface);
            let bounds = surface.debug_bounds("archive-volume-number");
            if bounds.is_some_and(|bounds| {
                bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
            }) {
                break;
            }
            let direction = bounds.map_or(-2., |bounds| {
                if bounds.top() < viewport.top() {
                    2.
                } else {
                    -2.
                }
            });
            surface.simulate_event(gpui_kit::ScrollEvent {
                position: gpui_kit::point(viewport.right() - px(12.), viewport.center().y),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., direction)),
                ..Default::default()
            });
        }
        for selector in [
            "archive-volume-preset",
            "archive-volume-number",
            "archive-volume-unit",
        ] {
            let bounds = surface
                .debug_bounds(selector)
                .expect("actual control wrapper");
            assert!(bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom());
            assert_eq!(bounds.size.height, px(32.));
        }
        for selector in ["archive-form-submit", "archive-form-cancel"] {
            let bounds = surface
                .debug_bounds(selector)
                .expect("nonshrinking official footer");
            assert!(bounds.bottom() <= dimensions.height);
            assert_eq!(bounds.size.height, px(32.));
        }
        click(&mut surface, "archive-volume-number");
        surface.simulate_input("2048");
        draw(&mut surface);
        surface.update(|_, cx| {
            view.update(cx, |form, cx| {
                assert_eq!(form.volume.read(cx).value().as_ref(), "2048");
                assert_eq!(
                    form.capture(cx).expect("actual typed MiB").0[0]
                        .create
                        .as_ref()
                        .expect("create")
                        .volume_bytes,
                    Some(2_147_483_648)
                );
            })
        });
        view.update_in(&mut surface, |form, window, cx| {
            form.volume_unit.focus_handle(cx).focus(window, cx)
        });
        surface.simulate_keystrokes("enter down enter");
        draw(&mut surface);
        surface.update(|_, cx| {
            view.update(cx, |form, cx| {
                assert_eq!(form.volume_unit.read(cx).selected_value(), Some(&"GiB"));
                assert_eq!(form.volume.read(cx).value().as_ref(), "2");
                assert_eq!(
                    form.submit_count, 0,
                    "Select Enter must not submit the outer form"
                );
                assert_eq!(
                    form.capture(cx)
                        .expect("same bytes after actual unit confirmation")
                        .0[0]
                        .create
                        .as_ref()
                        .expect("create")
                        .volume_bytes,
                    Some(2_147_483_648)
                );
            })
        });
    }
}

fn reveal_form_control(surface: &mut VisualTestContext, selector: &'static str) {
    for _ in 0..60 {
        draw(surface);
        let viewport = surface
            .debug_bounds("archive-form-body")
            .expect("original form viewport");
        let bounds = surface.debug_bounds(selector);
        if bounds.is_some_and(|bounds| {
            bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
        }) {
            return;
        }
        let direction = if bounds.is_some_and(|bounds| bounds.top() < viewport.top()) {
            2.
        } else {
            -2.
        };
        surface.simulate_event(gpui_kit::ScrollEvent {
            position: gpui_kit::point(viewport.right() - px(12.), viewport.center().y),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., direction)),
            ..Default::default()
        });
    }
    panic!("production form control {selector} was not reachable");
}

fn reveal_volume_controls(view: &Entity<OperationForm>, surface: &mut VisualTestContext) {
    if !view.read_with(surface, |form, _| form.space.options_open) {
        reveal_form_control(surface, "archive-options-toggle");
        click(surface, "archive-options-toggle");
        assert!(view.read_with(surface, |form, _| form.space.options_open));
    }
    reveal_form_control(surface, "archive-volume-number");
}

#[gpui_kit::test]
fn ref17_b3_01_actual_invalid_unit_switch_preserves_reason_and_correction(cx: &mut TestAppContext) {
    use nxtools_core::archive::{ArchiveVolumeSize, ArchiveVolumeUnit};
    for (value, reason) in [
        ("0", "分卷大小至少为 64 KiB，且必须是正整数。"),
        ("63", "分卷大小至少为 64 KiB，且必须是正整数。"),
        ("1.5", "分卷大小必须是正整数；请选择 KiB、MiB 或 GiB。"),
        ("18446744073709551615", "分卷大小超出可表示范围。"),
        ("18446744073709551616", "分卷大小超出可表示范围。"),
        (
            "65",
            "当前大小不能用所选单位的正整数精确表示。已保留原值；请保留单位或明确修改大小。",
        ),
    ] {
        let settings = ArchiveSettings {
            create_volume_size: Some(ArchiveVolumeSize {
                value: 64,
                unit: ArchiveVolumeUnit::KiB,
            }),
            ..Default::default()
        };
        let (view, mut surface) = mount(cx, settings);
        surface.simulate_resize(size(px(680.), px(600.)));
        reveal_volume_controls(&view, &mut surface);
        let input_id = view.read_with(&surface, |form, _| form.volume.entity_id());
        click(&mut surface, "archive-volume-number");
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input(value);
        draw(&mut surface);
        view.update_in(&mut surface, |form, window, cx| {
            form.volume_unit.focus_handle(cx).focus(window, cx)
        });
        surface.simulate_keystrokes("enter down enter");
        draw(&mut surface);
        view.read_with(&surface, |form, cx| {
            assert_eq!(
                form.volume.read(cx).value().as_ref(),
                value,
                "rejected switch retains actual text"
            );
            assert_eq!(form.volume_unit.read(cx).selected_value(), Some(&"KiB"));
            assert_eq!(form.volume.entity_id(), input_id);
            assert_eq!(
                form.error.as_deref(),
                Some(reason),
                "unit switch must preserve the real validation reason for {value}"
            );
            assert_eq!(form.submit_count, 0);
            assert!(!form.submitted);
        });
        reveal_form_control(&mut surface, "archive-volume-number");
        click(&mut surface, "archive-volume-number");
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input("2048");
        draw(&mut surface);
        view.update_in(&mut surface, |form, window, cx| {
            form.volume_unit.focus_handle(cx).focus(window, cx)
        });
        surface.simulate_keystrokes("enter down enter");
        draw(&mut surface);
        surface.update(|_, cx| {
            view.update(cx, |form, cx| {
                assert_eq!(form.volume.read(cx).value().as_ref(), "2");
                assert_eq!(form.volume_unit.read(cx).selected_value(), Some(&"MiB"));
                assert_eq!(form.volume.entity_id(), input_id);
                assert!(
                    form.error.is_none(),
                    "corrected exact input clears the error"
                );
                assert_eq!(
                    form.capture(cx).expect("corrected production form").0[0]
                        .create
                        .as_ref()
                        .expect("create")
                        .volume_bytes,
                    Some(2_097_152)
                );
                assert_eq!(form.submit_count, 0);
            })
        });
    }
}

#[gpui_kit::test]
fn ref17_b3_e1_actual_preset_and_step_controls_preserve_bytes_and_focus(cx: &mut TestAppContext) {
    for dimensions in [size(px(680.), px(600.)), size(px(480.), px(360.))] {
        let (view, mut surface) = mount(cx, ArchiveSettings::default());
        surface.simulate_resize(dimensions);
        reveal_volume_controls(&view, &mut surface);
        let input_id = view.read_with(&surface, |form, _| form.volume.entity_id());
        view.update_in(&mut surface, |form, window, cx| {
            form.volume_preset.focus_handle(cx).focus(window, cx)
        });
        surface.simulate_keystrokes("enter down down enter");
        draw(&mut surface);
        view.read_with(&surface, |form, cx| {
            assert_eq!(
                form.volume_preset.read(cx).selected_value(),
                Some(&"100 MiB")
            );
            assert_eq!(form.volume.read(cx).value().as_ref(), "100");
            assert_eq!(form.volume_unit.read(cx).selected_value(), Some(&"MiB"));
            assert_eq!(form.volume.entity_id(), input_id);
            assert_eq!(form.submit_count, 0);
        });
        for (button, value, bytes) in [
            (None, "100", 104_857_600),
            (Some("increment"), "101", 105_906_176),
            (Some("decrement"), "100", 104_857_600),
        ] {
            if let Some(button) = button {
                reveal_form_control(&mut surface, "archive-volume-number");
                let wrapper = surface
                    .debug_bounds("archive-volume-number")
                    .expect("number wrapper");
                let bounds = surface.update(|window, _| {
                    gpui_kit::base::test_support::snapshots(window)
                        .into_iter()
                        .find(|node| {
                            node.role() == Some(gpui_kit::Role::Button)
                                && node.path().last() == Some(&gpui_kit::ElementId::from(button))
                                && wrapper.contains(&node.bounds().center())
                        })
                        .expect("official NumberInput step button inside original wrapper")
                        .bounds()
                });
                surface.simulate_event(gpui_kit::MouseDownEvent {
                    position: bounds.center(),
                    button: gpui_kit::MouseButton::Left,
                    ..Default::default()
                });
                surface.simulate_event(gpui_kit::MouseUpEvent {
                    position: bounds.center(),
                    button: gpui_kit::MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                });
                draw(&mut surface);
                assert!(
                    surface.update(|window, cx| {
                        view.read(cx).volume.focus_handle(cx).is_focused(window)
                    }),
                    "step buttons keep focus on the original numeric input"
                );
            }
            surface.update(|_, cx| {
                view.update(cx, |form, cx| {
                    assert_eq!(form.volume.read(cx).value().as_ref(), value);
                    assert_eq!(form.volume.entity_id(), input_id);
                    assert_eq!(form.submit_count, 0);
                    assert!(!form.submitted);
                    assert_eq!(
                        form.capture(cx).expect("actual preset and deferred step").0[0]
                            .create
                            .as_ref()
                            .expect("create")
                            .volume_bytes,
                        Some(bytes)
                    );
                })
            });
        }
        let viewport = surface
            .debug_bounds("archive-form-body")
            .expect("scroll body");
        let number = surface
            .debug_bounds("archive-volume-number")
            .expect("reachable numeric row");
        assert!(number.top() >= viewport.top() && number.bottom() <= viewport.bottom());
        for selector in ["archive-form-submit", "archive-form-cancel"] {
            let footer = surface.debug_bounds(selector).expect("fixed footer action");
            assert_eq!(footer.size.height, px(32.));
            assert!(footer.top() >= viewport.bottom() && footer.bottom() <= dimensions.height);
        }
    }
}

#[path = "forms_ref20_tests.rs"]
mod ref20;
