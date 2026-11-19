//! ZIP-SFX interaction through the production form and official controls.
use super::*;

const SFX: &str = "archive-self-extracting";

fn captured(view: &Entity<OperationForm>, surface: &mut VisualTestContext) -> ArchiveJobRequest {
    view.update_in(surface, |form, _, cx| {
        form.capture(cx)
            .expect("valid immutable SFX intent")
            .0
            .remove(0)
    })
}

fn press(surface: &mut VisualTestContext, key: &str) {
    audit05_form_key(surface, key, true, false);
    audit05_form_key(surface, key, false, false);
    draw(surface);
}

#[gpui_kit::test]
fn ref20_pointer_names_and_immutable_capture(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    let entities = view.read_with(&surface, |form, _| {
        (form.name.entity_id(), form.format.entity_id())
    });
    assert!(
        !captured(&view, &mut surface)
            .create
            .expect("plain ZIP")
            .self_extracting
    );
    click(&mut surface, SFX);
    let request = captured(&view, &mut surface);
    let create = request.create.as_ref().expect("SFX options");
    assert!(create.self_extracting);
    assert_eq!(create.name, "input.exe");
    assert_eq!(create.format, ArchiveJobFormat::Zip);
    assert!(create.password.is_none());
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);
    view.read_with(&surface, |form, cx| {
        assert_eq!(entities, (form.name.entity_id(), form.format.entity_id()));
        assert_eq!(form.name.read(cx).value().as_str(), "input.exe");
        assert!(form.validation.is_none());
    });
    click(&mut surface, SFX);
    let plain = captured(&view, &mut surface).create.expect("restored ZIP");
    assert!(!plain.self_extracting);
    assert_eq!(plain.name, "input.zip");
    assert!(
        request
            .create
            .expect("captured snapshot remains SFX")
            .self_extracting
    );
}

#[gpui_kit::test]
fn ref20_checkbox_first_enter_space_repeat_and_focus(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, SFX);
    let focused = |surface: &mut VisualTestContext| {
        surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.label() == Some("创建自解压 ZIP（EXE）") && node.focused() == Some(true)
                })
        })
    };
    for _ in 0..40 {
        if focused(&mut surface) {
            break;
        }
        press(&mut surface, "tab");
    }
    assert!(
        focused(&mut surface),
        "official Checkbox participates in actual Tab traversal"
    );
    press(&mut surface, "space");
    assert!(
        !captured(&view, &mut surface)
            .create
            .expect("space toggled")
            .self_extracting
    );
    audit05_form_key(&mut surface, "enter", true, false);
    audit05_form_key(&mut surface, "enter", true, true);
    audit05_form_key(&mut surface, "enter", true, true);
    draw(&mut surface);
    assert!(
        !captured(&view, &mut surface)
            .create
            .expect("no click before key release")
            .self_extracting
    );
    audit05_form_key(&mut surface, "enter", false, false);
    draw(&mut surface);
    assert!(
        captured(&view, &mut surface)
            .create
            .expect("first Enter release toggles once after held repeats")
            .self_extracting
    );
    press(&mut surface, "enter");
    assert!(
        !captured(&view, &mut surface)
            .create
            .expect("new Enter toggled")
            .self_extracting
    );
    assert_eq!(
        view.read_with(&surface, |form, _| form.submit_count),
        0,
        "checkbox keys never submit the form"
    );
}

#[gpui_kit::test]
fn ref20_actual_format_select_clears_sfx_and_separate_names(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, SFX);
    view.update_in(&mut surface, |form, window, cx| {
        if let Draft::Create(draft) = &mut form.draft {
            draft.sources.push(DraftSource {
                path: r"C:\synthetic\other.txt".into(),
                directory: false,
                bytes: Some(4),
            });
            draft.separately = true;
        }
        let requests = form.capture(cx).expect("separate SFX requests").0;
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests
                .iter()
                .map(|request| request.create.as_ref().expect("create").name.as_str())
                .collect::<Vec<_>>(),
            ["input.exe", "other.exe"]
        );
        assert!(
            requests
                .iter()
                .all(|request| request.create.as_ref().expect("create").self_extracting)
        );
        form.format.focus_handle(cx).focus(window, cx);
    });
    draw(&mut surface);
    press(&mut surface, "enter");
    press(&mut surface, "down");
    press(&mut surface, "enter");
    let request = captured(&view, &mut surface);
    let create = request.create.expect("7z request");
    assert_eq!(create.format, ArchiveJobFormat::SevenZ);
    assert!(!create.self_extracting);
    assert_eq!(create.name, "input.7z");
    assert!(
        surface.debug_bounds(SFX).is_none(),
        "SFX choice is ZIP-only"
    );
    view.read_with(&surface, |form, cx| {
        assert_eq!(form.name.read(cx).value().as_str(), "input.7z");
        assert_eq!(form.submit_count, 0);
    });
}

#[gpui_kit::test]
fn ref20_password_is_preserved_with_visible_incompatibility(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    view.update_in(&mut surface, |form, window, cx| {
        form.password
            .update(cx, |state, cx| state.set_value("owned-secret", window, cx));
        form.confirm
            .update(cx, |state, cx| state.set_value("owned-secret", window, cx));
    });
    draw(&mut surface);
    click(&mut surface, SFX);
    view.read_with(&surface, |form, cx| {
        assert_eq!(form.password.read(cx).value().as_str(), "owned-secret");
        assert_eq!(form.confirm.read(cx).value().as_str(), "owned-secret");
        assert!(
            form.validation
                .as_ref()
                .is_some_and(|message| message.contains("自解压") && message.contains("密码"))
        );
    });
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("创建") && node.disabled() == Some(true))
    }));
    click(&mut surface, SFX);
    assert!(
        captured(&view, &mut surface)
            .create
            .expect("ordinary encrypted ZIP")
            .password
            .is_some()
    );
}

#[gpui_kit::test]
fn ref20_busy_escape_and_titlebar_close_keep_original_lifecycle(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, SFX);
    view.update_in(&mut surface, |form, _, cx| {
        form.enumerating = true;
        cx.notify();
    });
    draw(&mut surface);
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| {
                node.label() == Some("创建自解压 ZIP（EXE）") && node.disabled() == Some(true)
            })
    }));
    click(&mut surface, SFX);
    press(&mut surface, "space");
    assert!(
        captured(&view, &mut surface)
            .create
            .expect("busy retains intent")
            .self_extracting
    );
    view.update_in(&mut surface, |form, _, cx| {
        form.enumerating = false;
        cx.notify();
    });
    audit05_form_key(&mut surface, "escape", true, false);
    surface.run_until_parked();
    assert!(view.read_with(&surface, |form, _| form.retired));
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);

    let (view, mut surface) = mount(cx, ArchiveSettings::default());
    click(&mut surface, SFX);
    let position = surface
        .debug_bounds("archive-form-close")
        .expect("real titlebar close")
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
    surface.run_until_parked();
    assert!(view.read_with(&surface, |form, _| form.retired));
    assert_eq!(view.read_with(&surface, |form, _| form.submit_count), 0);
}
