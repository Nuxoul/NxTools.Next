//! Real launcher paint/identity and deferred theme observer regression.
use super::*;
use gpui_kit::{AppContext, TestAppContext};

#[gpui_kit::test]
fn auxiliary_quick_panel_rethemes_without_replacing_search_or_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let retained = Rc::new(std::cell::RefCell::new(None));
    let slot = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            QuickLaunchPanel::new(
                Vec::new(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 0,
                    focus_window_handle: 0,
                    process_id: 0,
                    process_name: "synthetic".into(),
                    window_title: String::new(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            )
        });
        *slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    window.refresh().expect("dark production search");
    let panel = retained.borrow().clone().expect("retained panel");
    let original_input = panel.read_with(window, |view, _| view.search_input.entity_id());
    let original_bounds = window
        .debug_bounds("quick-launch-search-group")
        .expect("search frame");
    assert!(window.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == gpui_kit::solid_background(Theme::zed_dark().canvas))
    }));
    let notifications = Rc::new(std::cell::Cell::new(0));
    let observed_notifications = notifications.clone();
    window.update(|_, cx| {
        cx.observe(&panel, move |_, _| {
            observed_notifications.set(observed_notifications.get() + 1);
        })
        .detach();
    });
    panel.update_in(window, |_, _, cx| {
        // Mutate while the consumer is leased: synchronous observer writeback panics.
        crate::theme::set_theme_preference(nxtools_core::ThemePreference::Light, cx);
    });
    window.run_until_parked();
    assert!(
        notifications.get() > 0,
        "global observer must notify the retained consumer on a later turn"
    );
    window.refresh().expect("light production search");
    assert!(window.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == gpui_kit::solid_background(Theme::light().canvas))
    }));
    assert_eq!(
        panel.read_with(window, |view, _| view.search_input.entity_id()),
        original_input
    );
    assert_eq!(
        window.debug_bounds("quick-launch-search-group"),
        Some(original_bounds)
    );
    assert_eq!(panel.read_with(window, |view, _| view.selected), 0);
}

#[gpui_kit::test]
fn auxiliary_quick_preview_rethemes_without_restarting_appearance(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (view, window) = cx.add_window_view(|_, _| {
        crate::quick_launch::preview::QuickLaunchPreview::new(
            crate::quick_launch::preview::QuickLaunchPreviewPayload {
                entry_id: "synthetic".into(),
                name: "合成预览".into(),
                target: "synthetic.txt".into(),
                detail: "只读预览".into(),
                created_at: None,
                modified_at: None,
                body: crate::quick_launch::preview::QuickLaunchPreviewBody::Text {
                    text: "合成预览正文".into(),
                    truncated: false,
                },
            },
            WeakEntity::new_invalid(),
        )
    });
    window.simulate_resize(size(px(400.0), px(410.0)));
    window.refresh().expect("dark preview");
    let generation = view.read_with(window, |view, _| view.appearance_generation);
    let original_bounds = window
        .debug_bounds("quick-launch-preview-content")
        .expect("production preview content");
    view.update_in(window, |_, _, cx| {
        crate::theme::set_theme_preference(nxtools_core::ThemePreference::Light, cx);
    });
    window.run_until_parked();
    window.refresh().expect("light preview");
    assert!(window.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == gpui_kit::solid_background(Theme::light().canvas))
    }));
    assert_eq!(
        view.read_with(window, |view, _| view.appearance_generation),
        generation
    );
    assert_eq!(
        window.debug_bounds("quick-launch-preview-content"),
        Some(original_bounds)
    );
}
