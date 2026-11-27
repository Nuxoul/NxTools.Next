//! Original ordinary-window owners under deferred material readiness.
use super::*;
use crate::archive::ordinary_material_test_support as support;
use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn material_browser_reuse_waits_and_missing_hwnd_retires_exact_host(cx: &mut TestAppContext) {
    let (owner, _) = tests::setup_unprepared(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\material.zip"), 0, vec![], cx);
        let id = *app.windows.keys().next().expect("original tab");
        app.activate_browser(id, cx);
        assert!(app.host.as_ref().expect("host").material_pending);
        assert!(app.activation.is_none(), "pending reuse cannot activate");
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        assert!(
            app.host.is_none(),
            "TestWindow cannot supply a native receipt"
        );
        assert!(app.windows.is_empty());
        assert!(app.activation.is_none());
    });
}

#[gpui_kit::test]
fn material_browser_stale_completion_cannot_retire_replacement(cx: &mut TestAppContext) {
    let (owner, _) = tests::setup_unprepared(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\one.zip"), 0, vec![], cx);
        let host = app.host.as_ref().expect("host");
        let old = (host.id, host.window);
        app.close_host(old.0, cx);
        app.open_canonical(PathBuf::from(r"C:\owned\two.zip"), 0, vec![], cx);
        let replacement = app.host.as_ref().expect("replacement").id;
        app.finish_browser_material(old.0, old.1, Err("material_timeout"), cx);
        assert_eq!(
            app.host.as_ref().expect("retained replacement").id,
            replacement
        );
    });
}

#[gpui_kit::test]
fn material_form_missing_hwnd_retires_only_original_form(cx: &mut TestAppContext) {
    let (owner, _) = tests::setup_unprepared(cx);
    owner.update(cx, |app, cx| {
        app.open_create_form(vec![], cx);
        assert_eq!(app.jobs.forms.len(), 1);
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, _| assert!(app.jobs.forms.is_empty()));
}

#[gpui_kit::test]
fn material_browser_current_receipt_selects_latest_tab_and_retains_runtime(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint) = tests::setup_unprepared(cx);
    let session = Uuid::new_v4();
    let (window, selected) = owner.update(cx, |app, cx| {
        support::facts(session, 1, cx);
        let first = PathBuf::from(r"C:\owned\first.zip");
        app.open_canonical(first.clone(), 0, vec![], cx);
        let selected = app.browser_id(&first).expect("first tab");
        app.open_canonical(PathBuf::from(r"C:\owned\second.zip"), 0, vec![], cx);
        app.activate_browser(selected, cx);
        let window = app.host.as_ref().expect("host").window;
        crate::ui::material::runtime::set_test_window_handle(window, 42, cx);
        assert!(app.activation.is_none());
        (window, selected)
    });
    cx.run_until_parked();
    let (id, command) = support::next(&endpoint);
    cx.update(|cx| {
        support::accept(&endpoint, id, command, cx);
        support::facts(session, 2, cx);
    });
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| {
        app.host.as_ref().expect("pending host").material_pending
    }));
    let (id, command) = support::next(&endpoint);
    cx.update(|cx| support::accept(&endpoint, id, command, cx));
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        let host = app.host.as_ref().expect("retained host");
        assert_eq!(host.window, window);
        assert!(!host.material_pending);
        assert_eq!(
            host.view
                .read_with(cx, |host, _| host.active)
                .expect("host view"),
            Some(selected)
        );
        assert_eq!(app.windows.len(), 2);
    });
    cx.update(|cx| support::facts(session, 3, cx));
    cx.run_until_parked();
    assert_eq!(
        support::next(&endpoint).1.window_handle,
        42,
        "shown window retains runtime coverage"
    );
}

#[gpui_kit::test]
fn material_form_receipt_preserves_original_form_entity(cx: &mut TestAppContext) {
    let (owner, endpoint) = tests::setup_unprepared(cx);
    let (id, view, window) = owner.update(cx, |app, cx| {
        support::facts(Uuid::new_v4(), 1, cx);
        app.open_create_form(vec![], cx);
        let (&id, slot) = app.jobs.forms.iter().next().expect("form");
        let result = (id, slot.view.clone(), slot.window);
        crate::ui::material::runtime::set_test_window_handle(slot.window, 43, cx);
        result
    });
    cx.run_until_parked();
    let (receipt, command) = support::next(&endpoint);
    cx.update(|cx| support::accept(&endpoint, receipt, command, cx));
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.get(&id).expect("same form");
        assert_eq!(slot.view, view);
        assert_eq!(slot.window, window);
    });
}
