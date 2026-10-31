//! Actual production Settings/MainWindow boundaries, scratch-only configuration.
use super::archive_page::ArchiveSettingsView;
use super::*;
use gpui_kit::{KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext, size};
use nxtools_core::AppConfig;
use nxtools_protocol::archive_shell_integration::ArchiveShellIntegrationAction;

fn mount(
    cx: &mut TestAppContext,
    path: PathBuf,
) -> (
    Entity<MainWindow>,
    Entity<ArchiveSettingsView>,
    VisualTestContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let notifications = cx.new(|_| NotificationCenter::default());
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        // MainWindow may hold an older config snapshot after a form remembered defaults.
        let view =
            cx.new(|cx| ArchiveSettingsView::new(AppConfig::default().archive, path, window, cx));
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main.archive_settings = Some(view.clone());
            main.shell.activate(MainModule::Archive);
            main
        });
        *slot.borrow_mut() = Some((main.clone(), view));
        gpui_kit::base::Root::new(main, window, cx)
    });
    let handle = window.update(|window, _| window.window_handle());
    let (main, view) = retained.borrow().clone().expect("production views");
    let window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(920.0), px(480.0)));
    (main, view, window)
}
fn click(window: &mut VisualTestContext, selector: &'static str) {
    window.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = window
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("mounted {selector}"));
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
}
fn select_page(window: &mut VisualTestContext, index: usize) {
    window.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&format!("0-{index}").into()))
            .expect("official Settings page")
            .bounds()
    });
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
}
#[gpui_kit::test]
fn archive_settings_navigation_save_failure_and_return_use_production_owners(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let original = std::fs::read(&path).expect("initial bytes");
    let (main, view, mut window) = mount(cx, path.clone());
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        std::fs::read(&path).expect("render bytes"),
        original,
        "render must not save"
    );
    select_page(&mut window, 2);
    click(&mut window, "archive-test-after-create");
    assert!(
        !AppConfig::load_from(&path)
            .expect("saved Switch")
            .archive
            .test_after_create
    );
    let input_id = view.read_with(&window, |view, _| view.exclusions[0].entity_id());
    click(&mut window, "main-module-其他工具");
    click(&mut window, "main-module-压缩");
    assert_eq!(
        main.read_with(&window, |main, _| main.shell.active_module()),
        MainModule::Archive
    );
    assert_eq!(
        view.read_with(&window, |view, _| view.exclusions[0].entity_id()),
        input_id
    );
    assert!(!view.read_with(&window, |view, _| view.settings.test_after_create));
    select_page(&mut window, 2);
    std::fs::write(&path, b"{broken").expect("corrupt fixture");
    click(&mut window, "archive-test-after-create");
    assert!(
        !view.read_with(&window, |view, _| view.settings.test_after_create),
        "failed save retains prior live value"
    );
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    assert!(
        window.debug_bounds("archive-settings-error").is_some(),
        "failure visible in original page"
    );
    assert_eq!(
        std::fs::read(path).expect("unchanged corruption"),
        b"{broken"
    );
}
#[gpui_kit::test]
fn archive_settings_real_keyboard_scroll_and_themes_keep_controls_reachable(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, _, mut window) = mount(cx, path.clone());
    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        window.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        for dimensions in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
            window.simulate_resize(dimensions);
            select_page(&mut window, 2);
            let mut reached = false;
            for _ in 0..30 {
                window.update(|window, cx| window.draw(cx).clear(cx));
                if let Some(bounds) = window.debug_bounds("archive-exclusions-save")
                    && bounds.top() >= px(72.0)
                    && bounds.bottom() <= dimensions.height
                {
                    reached = true;
                    break;
                }
                window.simulate_event(gpui_kit::ScrollWheelEvent {
                    position: point(dimensions.width - px(150.0), dimensions.height - px(90.0)),
                    delta: gpui_kit::ScrollDelta::Lines(point(0.0, -3.0)),
                    ..Default::default()
                });
            }
            assert!(reached, "official Settings scroll reaches exclusions Save");
        }
    }
    select_page(&mut window, 5);
    click(&mut window, "archive-copy-motw");
    window.simulate_keystrokes("tab shift-tab");
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("release"),
    });
    window.run_until_parked();
    assert!(
        AppConfig::load_from(&path)
            .expect("keyboard save")
            .archive
            .copy_motw,
        "one pointer then one keyboard activation"
    );
}

#[gpui_kit::test(iterations = 8)]
fn archive_settings_official_search_never_saves(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned search config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let before = std::fs::read(&path).expect("before");
    let (_, _, mut window) = mount(cx, path.clone());
    window.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let search = window.update(|window, _| {
        let inputs = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|node| {
                node.role() == Some(gpui_kit::Role::TextInput)
                    && node.path().contains(&"archive-official-settings".into())
                    && node.path().contains(&"settings-sidebar".into())
            })
            .collect::<Vec<_>>();
        assert_eq!(inputs.len(), 1, "one official Settings sidebar search");
        inputs[0].bounds()
    });
    window.simulate_click(search.center(), Modifiers::default());
    window.simulate_input("压缩后测试");
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| {
                node.role() == Some(gpui_kit::Role::TextInput)
                    && node.path().contains(&"archive-official-settings".into())
                    && node.path().contains(&"settings-sidebar".into())
                    && node.focused() == Some(true)
                    && node.value() == Some("压缩后测试")
            })
    }));
    assert!(window.debug_bounds("archive-test-after-create").is_some());
    window.simulate_keystrokes("ctrl-a");
    window.simulate_input("没有任何匹配的合成设置");
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.update(|window, _| {
        !gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("压缩后测试"))
    }));
    assert_eq!(std::fs::read(path).expect("after search"), before);
}

#[gpui_kit::test]
fn archive_settings_exclusion_editor_is_lossless_and_stream_levels_are_valid(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned presets");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    let patterns = vec![
        " keep-spaces.txt ".to_owned(),
        "semi;colon".into(),
        "line\nbreak".into(),
    ];
    view.update_in(&mut window, |view, window, cx| {
        for (index, pattern) in patterns.iter().enumerate() {
            if index >= view.exclusions.len() {
                view.add_exclusion(window, cx);
            }
            view.exclusions[index]
                .update(cx, |input, cx| input.set_value(pattern.clone(), window, cx));
        }
    });
    select_page(&mut window, 2);
    for _ in 0..20 {
        window.update(|window, cx| window.draw(cx).clear(cx));
        if window
            .debug_bounds("archive-exclusions-save")
            .is_some_and(|bounds| bounds.bottom() <= px(480.0))
        {
            break;
        }
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(750.0), px(390.0)),
            delta: gpui_kit::ScrollDelta::Lines(point(0.0, -4.0)),
            ..Default::default()
        });
    }
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved")
            .archive
            .exclude_patterns,
        patterns
    );
    for format in [
        nxtools_core::ArchiveCreateFormat::Gz,
        nxtools_core::ArchiveCreateFormat::Xz,
    ] {
        view.update(&mut window, |view, cx| {
            view.settings.create_format = format;
            cx.notify();
        });
        select_page(&mut window, 0);
        select_page(&mut window, 2);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let level = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.label() == Some("默认压缩级别: 标准")
                        && node.role() == Some(gpui_kit::Role::Button)
                })
                .expect("level Button")
                .bounds()
        });
        window.simulate_click(level.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.update(|window, _| !gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some("仅存储"))),
            "stream must not offer Store"
        );
        window.simulate_keystrokes("escape");
    }
}

#[gpui_kit::test]
fn phase_d_production_quick_actions_are_real_and_reachable(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned phase D config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let before = std::fs::read(&path).expect("before");
    let (_, _, mut window) = mount(cx, path.clone());
    window.update(|window, cx| window.draw(cx).clear(cx));
    for selector in ["archive-open", "archive-create", "archive-extract"] {
        let bounds = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("production Phase D action missing: {selector}"));
        assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(480.0));
        assert!(bounds.size.height >= px(32.0), "official Medium action");
    }
    assert_eq!(std::fs::read(path).expect("after render"), before);
}

#[gpui_kit::test]
fn archive_integration_page_saves_desired_revision_without_claiming_applied(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned integration config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    select_page(&mut window, 4);
    let mut found = false;
    for _ in 0..30 {
        window.update(|window, cx| window.draw(cx).clear(cx));
        if window.debug_bounds("archive-shell-cascade").is_some() {
            found = true;
            break;
        }
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(700.0), px(300.0)),
            delta: gpui_kit::ScrollDelta::Lines(point(0.0, -3.0)),
            ..Default::default()
        });
    }
    assert!(found, "official integration Switch mounted");
    click(&mut window, "archive-shell-cascade");
    let saved = AppConfig::load_from(&path)
        .expect("desired saved")
        .archive
        .shell_integration;
    assert!(saved.classic_cascade, "desired intent persisted");
    assert_eq!(
        saved.desired_revision, 1,
        "every desired change bumps revision"
    );
    assert!(
        view.read_with(&window, |view, _| view.integration.is_none()),
        "saving desired intent never fabricates an observed applied state"
    );
    let before_apply = std::fs::read(&path).expect("bytes before apply");
    // No archive owner is registered in this isolated mount: the explicit request must
    // surface a non-applied outcome and never write configuration or claim success.
    view.update(&mut window, |view, cx| {
        view.request_integration(ArchiveShellIntegrationAction::Apply, cx)
    });
    window.run_until_parked();
    assert_eq!(
        std::fs::read(&path).expect("bytes after apply"),
        before_apply
    );
    assert!(view.read_with(&window, |view, _| matches!(
        view.integration,
        Some(crate::archive::shell_integration::IntegrationOutcome::NotDelivered(_))
    ) && !view.integration_pending));
}

fn reveal(window: &mut VisualTestContext, selector: &'static str) {
    window.update(|window, cx| window.draw(cx).clear(cx));
    let viewport = window
        .debug_bounds("archive-settings-viewport")
        .expect("production Settings viewport");
    let position = point(
        viewport.right() - px(12.),
        viewport.top() + viewport.size.height / 2.,
    );
    window.simulate_event(gpui_kit::ScrollWheelEvent {
        position,
        delta: gpui_kit::ScrollDelta::Lines(point(0., 100.)),
        ..Default::default()
    });
    for _ in 0..60 {
        window.update(|window, cx| window.draw(cx).clear(cx));
        if window.debug_bounds(selector).is_some_and(|bounds| {
            bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
        }) {
            return;
        }
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position,
            delta: gpui_kit::ScrollDelta::Lines(point(0., -2.)),
            ..Default::default()
        });
    }
    panic!(
        "reachable production setting {selector}: target={:?}, viewport={viewport:?}",
        window.debug_bounds(selector)
    );
}

#[gpui_kit::test]
fn ordinary_archive_settings_have_real_persistent_controls_and_reopen(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned preference config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    for (page, selector, field, default) in [
        (
            0,
            "archive-escape-closes-window",
            "escape_closes_window",
            false,
        ),
        (
            1,
            "archive-auto-enter-single-root",
            "auto_enter_single_root",
            true,
        ),
        (1, "archive-open-after-extract", "open_after_extract", true),
        (
            1,
            "archive-keep-extract-progress-open",
            "keep_extract_progress_open",
            false,
        ),
        (
            1,
            "archive-smart-single-file-folder",
            "smart_single_file_folder",
            false,
        ),
        (
            2,
            "archive-keep-create-progress-open",
            "keep_create_progress_open",
            false,
        ),
        (3, "archive-show-parent-row", "show_parent_row", true),
        (3, "archive-show-grid-lines", "show_grid_lines", false),
        (3, "archive-full-row-select", "full_row_select", true),
        (3, "archive-single-click-open", "single_click_open", false),
        (3, "archive-show-tree", "show_tree", true),
        (3, "archive-show-full-path", "show_full_path", false),
        (3, "archive-auto-expand-tree", "auto_expand_tree", true),
    ] {
        select_page(&mut window, page);
        reveal(&mut window, selector);
        click(&mut window, selector);
        let saved = AppConfig::load_from(&path).expect("production switch save");
        assert_eq!(
            serde_json::to_value(saved.archive).expect("settings")[field],
            serde_json::json!(!default),
            "{field}"
        );
    }
    let expected = view.read_with(&window, |view, _| view.settings.clone());
    let (_, reopened, _) = mount(cx, path.clone());
    assert_eq!(
        reopened.read_with(cx, |view, _| view.settings.clone()),
        expected
    );
    std::fs::write(&path, b"{broken").expect("save failure fixture");
    select_page(&mut window, 0);
    reveal(&mut window, "archive-escape-closes-window");
    click(&mut window, "archive-escape-closes-window");
    assert_eq!(
        view.read_with(&window, |view, _| view.settings.clone()),
        expected
    );
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    assert_eq!(std::fs::read(path).expect("failed bytes"), b"{broken");
}

#[gpui_kit::test]
fn exclusion_add_remove_invalid_and_failed_save_keep_exact_draft(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned rules");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let before = std::fs::read(&path).expect("before");
    let (_, view, mut window) = mount(cx, path.clone());
    select_page(&mut window, 2);
    reveal(&mut window, "archive-exclusions-add");
    click(&mut window, "archive-exclusions-add");
    assert_eq!(view.read_with(&window, |view, _| view.exclusions.len()), 3);
    view.update_in(&mut window, |view, window, cx| {
        view.exclusions[2].update(cx, |state, cx| state.set_value("../invalid", window, cx));
    });
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(std::fs::read(&path).expect("invalid not saved"), before);
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    view.update_in(&mut window, |view, window, cx| {
        view.exclusions[2].update(cx, |state, cx| {
            state.set_value(" space;line\nbreak ", window, cx)
        });
    });
    std::fs::write(&path, b"{broken").expect("failed persistence");
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        view.read_with(&window, |view, cx| view.exclusions[2]
            .read(cx)
            .value()
            .to_string()),
        " space;line\nbreak "
    );
    assert_eq!(
        view.read_with(&window, |view, _| view.settings.exclude_patterns.clone()),
        vec!["Thumbs.db", "desktop.ini"]
    );
    std::fs::write(&path, before).expect("repair fixture");
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved rules")
            .archive
            .exclude_patterns[2],
        " space;line\nbreak "
    );
    select_page(&mut window, 0);
    select_page(&mut window, 2);
    reveal(&mut window, "archive-exclusion-remove-0");
    click(&mut window, "archive-exclusion-remove-0");
    assert_eq!(view.read_with(&window, |view, _| view.exclusions.len()), 2);
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("remove is draft only")
            .archive
            .exclude_patterns
            .len(),
        3
    );
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("save deletion")
            .archive
            .exclude_patterns,
        vec!["desktop.ini", " space;line\nbreak "]
    );
}

#[gpui_kit::test]
fn association_select_all_and_clear_save_intent_only(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned association intent");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    select_page(&mut window, 4);
    reveal(&mut window, "archive-shell-select-all");
    click(&mut window, "archive-shell-select-all");
    let all = AppConfig::load_from(&path)
        .expect("all saved")
        .archive
        .shell_integration;
    assert_eq!(
        all.extensions,
        nxtools_core::archive::ARCHIVE_SHELL_EXTENSIONS
    );
    assert_eq!(all.desired_revision, 1);
    assert!(view.read_with(&window, |view, _| view.integration.is_none()));
    click(&mut window, "archive-shell-select-none");
    let cleared = AppConfig::load_from(&path)
        .expect("none saved")
        .archive
        .shell_integration;
    assert!(cleared.extensions.is_empty());
    assert_eq!(cleared.desired_revision, 2);
    assert!(view.read_with(&window, |view, _| view.integration.is_none()));
}

#[gpui_kit::test]
fn changing_one_setting_preserves_newer_form_defaults_in_same_archive_section(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned stale settings");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, _, mut window) = mount(cx, path.clone());
    AppConfig::update_section(&path, |config| {
        config.archive.open_after_extract = false;
        config.archive.keep_create_progress_open = true;
    })
    .expect("another archive owner saves");
    click(&mut window, "archive-open-in-nxtools");
    let saved = AppConfig::load_from(&path)
        .expect("single field merged")
        .archive;
    assert!(!saved.open_in_nxtools && !saved.open_after_extract && saved.keep_create_progress_open);
}

#[gpui_kit::test]
fn ui_ac01_untouched_rules_preserve_newer_form_rules(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned rule merge config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    AppConfig::update_section(&path, |config| {
        config.archive.exclude_patterns = vec!["new-form-rule.tmp".into()];
    })
    .expect("newer form-owned rules");
    click(&mut window, "archive-open-in-nxtools");
    assert_eq!(
        view.read_with(&window, |view, _| view.settings.exclude_patterns.clone()),
        vec!["new-form-rule.tmp"]
    );
    select_page(&mut window, 2);
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved")
            .archive
            .exclude_patterns,
        vec!["new-form-rule.tmp"],
        "untouched rule editors must not revert newer persisted rules"
    );
}

#[gpui_kit::test]
fn ui_ac01_dirty_rules_conflict_preserves_draft_and_external_rules(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned dirty rule config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    view.update_in(&mut window, |view, window, cx| {
        view.exclusions[0].update(cx, |input, cx| {
            input.set_value(" unsaved;rule\nline ", window, cx)
        });
    });
    AppConfig::update_section(&path, |config| {
        config.archive.exclude_patterns = vec!["external.tmp".into()];
    })
    .expect("external rules");
    click(&mut window, "archive-open-in-nxtools");
    select_page(&mut window, 2);
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("external remains")
            .archive
            .exclude_patterns,
        vec!["external.tmp"]
    );
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    assert_eq!(
        view.read_with(&window, |view, cx| view.exclusions[0]
            .read(cx)
            .value()
            .to_string()),
        " unsaved;rule\nline "
    );
    reveal(&mut window, "archive-exclusions-reload");
    click(&mut window, "archive-exclusions-reload");
    assert!(view.read_with(&window, |view, _| view.error.is_none()));
    assert_eq!(
        view.read_with(&window, |view, cx| view.exclusions[0]
            .read(cx)
            .value()
            .to_string()),
        "external.tmp"
    );
    view.update_in(&mut window, |view, window, cx| {
        view.exclusions[0].update(cx, |input, cx| input.set_value("confirmed.tmp", window, cx));
    });
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("save after explicit reload")
            .archive
            .exclude_patterns,
        vec!["confirmed.tmp"]
    );
}

#[gpui_kit::test]
fn ui_ac01_clean_rule_save_without_other_switch_adopts_newer_rules(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned direct rule merge");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    AppConfig::update_section(&path, |config| {
        config.archive.exclude_patterns = vec![" direct;rule\nline ".into()];
    })
    .expect("newer rules");
    select_page(&mut window, 2);
    reveal(&mut window, "archive-exclusions-save");
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("disk")
            .archive
            .exclude_patterns,
        vec![" direct;rule\nline "]
    );
    assert_eq!(
        view.read_with(&window, |view, cx| view
            .exclusions
            .iter()
            .map(|input| input.read(cx).value().to_string())
            .collect::<Vec<_>>()),
        vec![" direct;rule\nline "]
    );
}

#[gpui_kit::test]
fn shell_settings_completion_controls_are_real_and_save_command_choices(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned integration UI");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    select_page(&mut window, 4);
    reveal(&mut window, "archive-default-apps");
    click(&mut window, "archive-default-apps");
    assert!(
        view.read_with(&window, |view, _| view.error.is_some())
            || window.debug_bounds("archive-default-apps-error").is_some(),
        "unavailable native owner must report failure"
    );
    reveal(&mut window, "archive-menu-command-create");
    click(&mut window, "archive-menu-command-create");
    let saved = AppConfig::load_from(&path).expect("saved menu");
    let json = serde_json::to_value(saved.archive.shell_integration).expect("settings");
    assert!(
        json["menu_commands"]
            .as_array()
            .expect("commands")
            .contains(&serde_json::json!("create"))
    );
    reveal(&mut window, "archive-menu-none");
    click(&mut window, "archive-menu-none");
    assert!(
        AppConfig::load_from(&path)
            .expect("clear")
            .archive
            .shell_integration
            .menu_commands
            .is_empty()
    );
    reveal(&mut window, "archive-menu-all");
    click(&mut window, "archive-menu-all");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("all")
            .archive
            .shell_integration
            .menu_commands
            .len(),
        7
    );
    let (_, reopened, _) = mount(cx, path.clone());
    assert_eq!(
        reopened.read_with(cx, |view, _| view.settings.shell_integration.clone()),
        AppConfig::load_from(&path)
            .expect("reopen")
            .archive
            .shell_integration
    );
}

#[gpui_kit::test]
fn shell_settings_dropdown_keyboard_minimum_themes_and_failed_save(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned shell settings");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        window.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        select_page(&mut window, 4);
        for selector in [
            "archive-default-apps",
            "archive-menu-command-create",
            "archive-menu-all",
            "archive-menu-none",
            "archive-shell-apply",
        ] {
            reveal(&mut window, selector);
        }
    }
    // Public Settings dropdown owns the keyboard choice and its one save callback.
    select_page(&mut window, 0);
    select_page(&mut window, 4);
    window.update(|window, cx| window.draw(cx).clear(cx));
    let search = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.role() == Some(gpui_kit::Role::TextInput))
            .expect("official settings search")
            .bounds()
    });
    window.simulate_click(search.center(), Modifiers::default());
    window.simulate_input("双击动作");
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    let choice = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("双击动作: 在 NxTools 中打开"))
            .expect("actual dropdown")
            .bounds()
    });
    window.simulate_click(choice.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("解压到同名文件夹（确认后开始）"))),
        "actual popup options mounted"
    );
    window.simulate_keystrokes("down down down enter");
    window.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("keyboard save")
            .archive
            .shell_integration
            .double_click_action,
        nxtools_core::archive::ArchiveDoubleClickAction::Name
    );
    window.simulate_click(search.center(), Modifiers::default());
    window.simulate_keystrokes("ctrl-a backspace");
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    // A different owner changes another integration field; this page changes only its own field.
    AppConfig::update_section(&path, |config| {
        config.archive.shell_integration.menu_layout =
            nxtools_core::archive::ArchiveMenuLayout::Flat
    })
    .expect("other integration save");
    reveal(&mut window, "archive-menu-command-create");
    click(&mut window, "archive-menu-command-create");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("field merge")
            .archive
            .shell_integration
            .menu_layout,
        nxtools_core::archive::ArchiveMenuLayout::Flat
    );
    let before = view.read_with(&window, |view, _| view.settings.shell_integration.clone());
    std::fs::write(&path, b"{broken").expect("failed config");
    click(&mut window, "archive-menu-command-create");
    assert_eq!(
        view.read_with(&window, |view, _| view.settings.shell_integration.clone()),
        before
    );
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
}

#[gpui_kit::test]
fn ui_remaining_recent_settings_use_actual_controls_and_failed_clear(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("owned settings");
    let path = directory.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    select_page(&mut window, 0);
    reveal(&mut window, "archive-remember-recent");
    click(&mut window, "archive-remember-recent");
    assert!(
        !AppConfig::load_from(&path)
            .expect("saved")
            .archive
            .remember_recent
    );
    reveal(&mut window, "archive-recent-clear");
    click(&mut window, "archive-recent-clear");
    window.run_until_parked();
    assert!(
        view.read_with(&window, |view, _| view.error.is_some()),
        "missing owner is an error, not a fake clear"
    );
    let (_, reopened, _) = mount(cx, path);
    assert!(!reopened.read_with(cx, |view, _| view.settings.remember_recent));
}

#[gpui_kit::test]
fn ui_remaining_temp_controls_save_reopen_invalid_and_changed_field_merge(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("owned temp settings");
    let path = directory.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    for theme in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        window.update(|_, cx| crate::theme::set_theme_preference(theme, cx));
        select_page(&mut window, 5);
        for selector in [
            "archive-temp-browse",
            "archive-temp-save",
            "archive-temp-default",
            "archive-temp-child-exit",
        ] {
            reveal(&mut window, selector);
        }
    }
    select_page(&mut window, 5);
    reveal(&mut window, "archive-temp-save");
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.temp_path.update(cx, |input, cx| {
                input.set_value(r"C:\owned\new-temp", window, cx)
            })
        })
    });
    AppConfig::update_section(&path, |config| config.archive.temp.retention_hours = 72)
        .expect("other field");
    click(&mut window, "archive-temp-save");
    let saved = AppConfig::load_from(&path).expect("saved").archive;
    assert_eq!(saved.temp.base_directory, r"C:\owned\new-temp");
    assert_eq!(saved.temp.retention_hours, 72);
    reveal(&mut window, "archive-temp-child-exit");
    click(&mut window, "archive-temp-child-exit");
    assert!(
        AppConfig::load_from(&path)
            .expect("switch")
            .archive
            .temp
            .cleanup_on_child_exit
    );
    let before = std::fs::read(&path).expect("valid bytes");
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.temp_path.update(cx, |input, cx| {
                input.set_value(r"\\server\share", window, cx)
            })
        })
    });
    reveal(&mut window, "archive-temp-save");
    click(&mut window, "archive-temp-save");
    assert_eq!(std::fs::read(&path).expect("invalid preserved"), before);
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.temp_path.update(cx, |input, cx| {
                input.set_value(r"C:\owned\local-conflict", window, cx)
            })
        })
    });
    AppConfig::update_section(&path, |config| {
        config.archive.temp.base_directory = r"D:\owned\external".into()
    })
    .expect("other owner path");
    click(&mut window, "archive-temp-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("conflict preserved")
            .archive
            .temp
            .base_directory,
        r"D:\owned\external"
    );
    assert_eq!(
        view.read_with(&window, |view, cx| view
            .temp_path
            .read(cx)
            .value()
            .to_string()),
        r"C:\owned\local-conflict"
    );
    AppConfig::update_section(&path, |config| {
        config.archive.temp.base_directory = r"C:\owned\new-temp".into()
    })
    .expect("restore fixture");
    let (_, reopened, _) = mount(cx, path);
    assert_eq!(
        reopened.read_with(cx, |view, _| view.settings.temp.base_directory.clone()),
        r"C:\owned\new-temp"
    );
}

#[gpui_kit::test]
fn ui_remaining_temp_picker_late_selection_preserves_new_draft_and_cancel_never_saves(
    cx: &mut TestAppContext,
) {
    let directory = tempfile::tempdir().expect("owned picker settings");
    let config = directory.path().join("config.json");
    AppConfig::default().save_to(&config).expect("seed");
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| {
        crate::archive::register(
            Rc::new(bridge),
            AppConfig::default().archive,
            directory.path().join("ui-state.json"),
            cx,
        )
    });
    let (_, view, mut window) = mount(cx, config.clone());
    select_page(&mut window, 5);
    reveal(&mut window, "archive-temp-browse");
    click(&mut window, "archive-temp-browse");
    let request = endpoint
        .try_receive_command_for_contract_test()
        .expect("original picker");
    assert!(matches!(
        request.message,
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::PickDirectory(_)
        )
    ));
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.temp_path.update(cx, |input, cx| {
                input.set_value(r"C:\owned\newer-draft", window, cx)
            })
        })
    });
    cx.update(|cx| {
        crate::archive::handle(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: request.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(r"C:\owned\late-picked".into()),
                },
            },
            cx,
        )
    });
    window.run_until_parked();
    assert_eq!(
        view.read_with(&window, |view, cx| view
            .temp_path
            .read(cx)
            .value()
            .to_string()),
        r"C:\owned\newer-draft"
    );
    assert!(
        AppConfig::load_from(&config)
            .expect("not saved")
            .archive
            .temp
            .base_directory
            .is_empty()
    );
    reveal(&mut window, "archive-temp-browse");
    click(&mut window, "archive-temp-browse");
    let request = endpoint
        .try_receive_command_for_contract_test()
        .expect("second picker");
    cx.update(|cx| {
        crate::archive::handle(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: request.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    window.run_until_parked();
    assert_eq!(
        view.read_with(&window, |view, cx| view
            .temp_path
            .read(cx)
            .value()
            .to_string()),
        r"C:\owned\newer-draft"
    );
}

#[gpui_kit::test]
fn ui_remaining_recent_actual_list_opens_original_read_and_clear_is_durable(
    cx: &mut TestAppContext,
) {
    let directory = tempfile::tempdir().expect("owned history UI");
    let config_path = directory.path().join("config.json");
    AppConfig::default().save_to(&config_path).expect("config");
    let archive_path = directory.path().join("recent.zip");
    std::fs::write(&archive_path, b"fixture").expect("owned path");
    let ui_path = directory.path().join("ui-state.json");
    let seed = serde_json::json!({"version":1,"geometry":{"x":0,"y":0,"width":960,"height":640,"maximized":false},"widths":[240.,112.,112.,112.,192.,112.,80.,112.,112.,280.],"visible":[true,true,true,true,true,false,false,false,false,false],"show_tree":true,"show_comment":false,"sort":"name","descending":false,"recent":[archive_path]});
    std::fs::write(&ui_path, serde_json::to_vec(&seed).expect("state")).expect("seed history");
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| {
        crate::archive::register(
            Rc::new(bridge),
            AppConfig::default().archive,
            ui_path.clone(),
            cx,
        )
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if cx.update(|cx| crate::archive::recent_snapshot(cx).is_ok()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let (_, view, mut window) = mount(cx, config_path);
    select_page(&mut window, 0);
    reveal(&mut window, "archive-recent-open-0");
    click(&mut window, "archive-recent-open-0");
    let mut pending_read = None;
    for _ in 0..200 {
        if let Ok(request) = endpoint.try_receive_command_for_contract_test() {
            pending_read = Some(request);
            break;
        }
        window.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let request = pending_read.expect("bounded recent-open read admission");
    assert!(matches!(
        request.message,
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::ArchiveBrowserRead(_)
        )
    ));
    reveal(&mut window, "archive-recent-clear");
    click(&mut window, "archive-recent-clear");
    for _ in 0..100 {
        window.run_until_parked();
        if serde_json::from_slice::<serde_json::Value>(&std::fs::read(&ui_path).expect("state"))
            .expect("JSON")["recent"]
            == serde_json::json!([])
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(&ui_path).expect("durable"))
            .expect("JSON")["recent"],
        serde_json::json!([])
    );
    window.run_until_parked();
    assert!(view.read_with(&window, |view, _| view.error.is_none()));
}
