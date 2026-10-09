//! Render-bound checks of the real link panel, not only height arithmetic.
use super::link_detector::{DetectedLink, DetectedLinkKind};
use super::link_panel::{LINK_PANEL_WIDTH, LinkPanelView, link_panel_height};
use gpui::{AppContext, TestAppContext, px, size};
use gpui_component::Root;

#[gpui::test]
fn link_panel_collapsed_and_expanded_render_within_the_fixed_client(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let links = [
        DetectedLinkKind::Web,
        DetectedLinkKind::LocalFile,
        DetectedLinkKind::LocalDirectory,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| DetectedLink {
        display: format!("test-{index}"),
        copy_value: format!("test-{index}"),
        kind,
    })
    .collect::<Vec<_>>();
    let collapsed = link_panel_height(links.len(), false);
    let expanded = link_panel_height(links.len(), true);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(LINK_PANEL_WIDTH), px(collapsed)));
        let handle = window.window_handle();
        let view = cx.new(|_| LinkPanelView {
            links,
            application: gpui::WeakEntity::new_invalid(),
            expanded: false,
            auto_hide_ms: 6_000,
            window_handle: Some(handle),
            auto_open_browser: false,
            copy_only: true,
        });
        Root::new(view, window, cx)
    });
    window.refresh().expect("render collapsed production panel");
    let first = window.debug_bounds("link-panel-row-0").expect("first row");
    let expander = window
        .debug_bounds("link-panel-expander")
        .expect("collapsed expander");
    let footer = window.debug_bounds("link-panel-footer").expect("footer");
    assert_eq!(first.size.height, px(28.0));
    assert_eq!(expander.size.height, px(24.0));
    assert_eq!(footer.size.height, px(22.0));
    assert!(first.bottom() <= expander.top());
    assert!(expander.bottom() <= footer.top());
    assert!(footer.bottom() <= px(collapsed));
    assert!(window.debug_bounds("link-panel-row-1").is_none());

    window.simulate_click(expander.center(), gpui::Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render expanded production panel");
    let third = window
        .debug_bounds("link-panel-row-2")
        .expect("expanded third row");
    let footer = window
        .debug_bounds("link-panel-footer")
        .expect("expanded footer");
    assert!(third.bottom() <= footer.top());
    assert!(footer.bottom() <= px(expanded));
}

#[gpui::test]
fn link_panel_row_keyup_uses_copy_only_route_without_application(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (_root, window) = cx.add_window_view(|window, cx| {
        window.resize(size(px(LINK_PANEL_WIDTH), px(link_panel_height(1, false))));
        let view = cx.new(|_| LinkPanelView {
            links: vec![DetectedLink {
                display: "https://example.invalid".into(),
                copy_value: "https://example.invalid".into(),
                kind: DetectedLinkKind::Web,
            }],
            application: gpui::WeakEntity::new_invalid(),
            expanded: false,
            auto_hide_ms: 6_000,
            window_handle: Some(window.window_handle()),
            auto_open_browser: false,
            copy_only: true,
        });
        Root::new(view, window, cx)
    });
    window.refresh().expect("render focusable link row");
    let bounds = window
        .debug_bounds("link-panel-row-0")
        .expect("link row hitbox");
    assert_eq!(bounds.size.height, px(28.0));
    window.update(|window, _| window.focus_next());
    window
        .refresh()
        .expect("focus link row through actual Tab order");
    window.simulate_keystrokes("enter");
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("Enter stroke"),
    });
    window.run_until_parked();
    // A copy-only row closes itself before dispatch; no live clipboard application is required.
    assert!(cx.update(|cx| cx.read_from_clipboard().is_some()));
}
