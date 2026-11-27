use super::*;
use crate::ui::corner_surface_tests::{assert_exterior, init, layers, project};
use gpui_kit::TestAppContext;
use nxtools_core::{MaterialStyle, ThemePreference};
use nxtools_protocol::window_material::EffectiveWindowMaterial;

#[gpui_kit::test]
fn corner_surface_quick_root_header_and_expansion_keep_material_and_identity(
    cx: &mut TestAppContext,
) {
    init(cx);
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (root, surface) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
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
        *retained.borrow_mut() = Some(panel.clone());
        crate::ui::popup_surface::root(panel, window, cx)
    });
    surface.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    surface.refresh().expect("production QuickLaunchPanel");
    let panel = slot.borrow().clone().expect("retained panel");
    let root_id = root.entity_id();
    let view_id = panel.entity_id();
    let input_id = panel.read_with(surface, |panel, _| panel.search_input.entity_id());
    panel.update_in(surface, |panel, window, cx| {
        panel.search_input.focus_handle(cx).focus(window, cx);
        panel.selected = 2;
    });
    let focus = surface.update(|window, cx| window.focused(cx));
    let input_bounds = surface.debug_bounds("quick-launch-search-group");
    for preference in [ThemePreference::Dark, ThemePreference::Light] {
        panel.update_in(surface, |_, _, cx| {
            crate::theme::set_appearance_preferences(preference, MaterialStyle::Acrylic, cx);
        });
        surface.run_until_parked();
        let expected = layers(surface).opaque;
        assert_exterior(surface, Some(expected));
        project(surface, EffectiveWindowMaterial::Acrylic);
        let expected = layers(surface).backdrop();
        assert_exterior(surface, Some(expected));
        project(surface, EffectiveWindowMaterial::AcrylicOpaque);
        let expected = layers(surface).opaque;
        assert_exterior(surface, Some(expected));
        panel.update_in(surface, |_, _, cx| {
            crate::theme::set_appearance_preferences(preference, MaterialStyle::Solid, cx);
        });
        surface.run_until_parked();
        assert_exterior(surface, None);
        assert_eq!(root.entity_id(), root_id);
        assert_eq!(
            root.read_with(surface, |root, _| root.view().entity_id()),
            view_id
        );
        assert_eq!(
            panel.read_with(surface, |panel, _| panel.search_input.entity_id()),
            input_id
        );
        assert_eq!(panel.read_with(surface, |panel, _| panel.selected), 2);
        assert_eq!(surface.update(|window, cx| window.focused(cx)), focus);
        assert_eq!(
            surface.debug_bounds("quick-launch-search-group"),
            input_bounds
        );
    }
    let header = surface
        .debug_bounds("quick-launch-search-header")
        .expect("touching header");
    surface.update(|window, _| {
        let scale = window.scale_factor();
        let header = window
            .painted_quads()
            .into_iter()
            .find(|quad| quad.bounds == header.scale(scale) && !quad.background.is_transparent())
            .expect("actual Solid header quad");
        assert_eq!(
            header.corner_radii,
            gpui_kit::Corners::all(px(7.0)).scale(scale)
        );
    });
    panel.update_in(surface, |panel, _, cx| {
        panel.results_visible = true;
        cx.notify();
    });
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    assert_exterior(surface, None);
    surface.simulate_scale_factor_change(1.25);
    assert_exterior(surface, None);
    panel.update_in(surface, |panel, _, cx| {
        panel.results_visible = false;
        cx.notify();
    });
    surface.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    assert_exterior(surface, None);
}

/// Whether a bordered frame is painted inside the search group (the Kit input's own outline).
fn search_frame(surface: &mut gpui_kit::VisualTestContext) -> bool {
    surface.refresh().expect("production search header");
    let group = surface
        .debug_bounds("quick-launch-search-group")
        .expect("search group");
    surface.update(|window, _| {
        let group = group.scale(window.scale_factor());
        window.painted_quads().iter().any(|quad| {
            group.intersect(&quad.bounds) == quad.bounds
                && quad.border_color.a > 0.0
                && quad.border_widths.left.0 > 0.0
        })
    })
}

#[gpui_kit::test]
fn corner_surface_quick_search_is_the_panel_itself_on_every_frosted_panel(cx: &mut TestAppContext) {
    init(cx);
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
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
        crate::ui::popup_surface::root(panel, window, cx)
    });
    surface.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    surface.refresh().expect("production QuickLaunchPanel");
    let input_bounds = surface.debug_bounds("quick-launch-search-group");
    assert!(input_bounds.is_some());
    // Windows 11 system acrylic and, below, the Windows 10 host acrylic.
    for preference in [ThemePreference::Light, ThemePreference::Dark] {
        surface.update(|_, cx| {
            crate::theme::set_appearance_preferences(preference, MaterialStyle::Acrylic, cx)
        });
        surface.run_until_parked();
        assert!(
            !search_frame(surface),
            "{preference:?}: Windows 11 frosted panel is the search field"
        );
        assert_eq!(
            surface.debug_bounds("quick-launch-search-group"),
            input_bounds
        );
    }
    surface.update(|_, cx| {
        let mut facts = crate::ui::material::UiMaterialCapabilities::default();
        facts.connected();
        assert!(facts.apply(
            nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
                correlation_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                revision: 1,
                supported: true,
                transparency_enabled: true,
                high_contrast: false,
                energy_saver: false,
                remote_session: false,
                native_error_code: 0,
                system_backdrop: false,
                rounded_corners: false,
            }
        ));
        cx.set_global(facts);
    });
    surface.run_until_parked();
    for preference in [ThemePreference::Dark, ThemePreference::Light] {
        surface.update(|_, cx| {
            crate::theme::set_appearance_preferences(preference, MaterialStyle::Acrylic, cx)
        });
        surface.run_until_parked();
        assert!(
            !search_frame(surface),
            "{preference:?}: one surface, no inner frame"
        );
        // The text keeps its place: only the frame is dropped.
        assert_eq!(
            surface.debug_bounds("quick-launch-search-group"),
            input_bounds
        );
    }
    surface.update(|_, cx| {
        crate::theme::set_appearance_preferences(ThemePreference::Light, MaterialStyle::Solid, cx)
    });
    surface.run_until_parked();
    assert!(
        search_frame(surface),
        "Solid keeps the original framed input"
    );
}
