//! Bounded TestWindow scenes; no system clipboard, native windows or user history.
use super::*;
use gpui_kit::base::Root;
use gpui_kit::{AppContext, Corners, Decorations, TestAppContext, Tiling, VisualTestContext};
use nxtools_core::{MaterialStyle, ThemePreference};
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason, MaterialSurface,
    SystemMaterialCapabilitiesChanged,
};

pub(crate) fn init(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.update(|cx| {
        let facts = cx.default_global::<material::UiMaterialCapabilities>();
        facts.connected();
        assert!(facts.apply(SystemMaterialCapabilitiesChanged {
            correlation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            revision: 1,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        }));
    });
}

pub(crate) fn project(surface: &mut VisualTestContext, effective: EffectiveWindowMaterial) {
    surface.update(|window, cx| {
        let handle = window.window_handle();
        material::runtime::set_test_window_handle(handle, 81, cx);
        let command =
            material::command(81, MaterialSurface::Backdrop, cx).expect("synthetic facts");
        material::paint::publish(
            command,
            AppliedWindowMaterial {
                session_id: command.session_id,
                facts_revision: command.facts_revision,
                window_handle: command.window_handle,
                generation: command.generation,
                style: command.style,
                surface: command.surface,
                dark: command.dark,
                effective,
                fallback_reason: if effective == EffectiveWindowMaterial::AcrylicOpaque {
                    MaterialFallbackReason::Unsupported
                } else {
                    MaterialFallbackReason::None
                },
                native_error_code: 0,
            },
            cx,
        );
    });
}

pub(crate) fn layers(surface: &mut VisualTestContext) -> crate::theme::material::Layers {
    surface.update(|_, cx| crate::theme::material::layers(cx))
}

/// The Root foundation must be one 8 DIP rounded quad carrying `fill`, or the official Root fill
/// (the palette background) when `None`.
pub(crate) fn assert_exterior(surface: &mut VisualTestContext, fill: Option<gpui_kit::Hsla>) {
    surface.refresh().expect("production popup scene");
    surface.update(|window, cx| {
        let scale = window.scale_factor();
        let viewport = window.viewport_size();
        let width = window.pixel_snap(viewport.width).as_f32();
        let height = window.pixel_snap(viewport.height).as_f32();
        let quads = window.painted_quads();
        let expected = fill
            .unwrap_or(gpui_kit::component::Theme::global(cx).colors.background)
            .to_rgb();
        let foundation = quads
            .iter()
            .find(|quad| {
                (quad.bounds.size.width.as_f32() / scale - width).abs() < 0.01
                    && (quad.bounds.size.height.as_f32() / scale - height).abs() < 0.01
                    && !quad.background.is_transparent()
            })
            .expect("actual Root foundation quad");
        assert_eq!(foundation.corner_radii, Corners::all(px(8.0)).scale(scale));
        let actual = foundation
            .background
            .as_solid()
            .expect("actual Root foundation must be a solid tint")
            .to_rgb();
        for (actual, expected) in [
            (actual.r, expected.r),
            (actual.g, expected.g),
            (actual.b, expected.b),
            (actual.a, expected.a),
        ] {
            assert!(
                (actual - expected).abs() < 0.000001,
                "Root paints the material foundation: {actual} != {expected}"
            );
        }
        for quad in quads
            .iter()
            .filter(|quad| !quad.background.is_transparent() || quad.border_color.a > 0.0)
        {
            let left = quad.bounds.left().as_f32() / scale;
            let top = quad.bounds.top().as_f32() / scale;
            let right = width - quad.bounds.right().as_f32() / scale;
            let bottom = height - quad.bounds.bottom().as_f32() / scale;
            for (x, y, radius) in [
                (left, top, quad.corner_radii.top_left),
                (right, top, quad.corner_radii.top_right),
                (left, bottom, quad.corner_radii.bottom_left),
                (right, bottom, quad.corner_radii.bottom_right),
            ] {
                if (0.0..8.0).contains(&x) && (0.0..8.0).contains(&y) {
                    let snap_tolerance = if x > 0.0 || y > 0.0 { 0.5 / scale } else { 0.0 };
                    assert!(
                        radius.as_f32() / scale >= 8.0 - x.max(y) - snap_tolerance - 0.01,
                        "touching child must not repaint the exterior square: {quad:?}"
                    );
                }
            }
        }
        // The global radius follows the system rounding reported by `init`, for every style.
        assert_eq!(
            gpui_kit::component::Theme::global(cx).window_radius,
            px(8.0)
        );
    });
}

#[gpui_kit::test]
fn corner_surface_clipboard_root_and_shell_keep_material_and_identity(cx: &mut TestAppContext) {
    init(cx);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("isolated synthetic history");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("history.db"),
        directory.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("synthetic worker");
    let held_worker = worker.clone();
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (root, surface) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                ClipboardHistoryPage {
                    items: Vec::new(),
                    total: 0,
                    has_more: false,
                },
                vec![ClipboardCategory {
                    id: BUILTIN_ALL_CATEGORY_ID.into(),
                    name: "全部".into(),
                    kind: ClipboardCategoryKind::All,
                    sort_order: 0,
                }],
                HashMap::new(),
                Vec::new(),
                ClipboardModifierActions::default(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 0,
                    focus_window_handle: 0,
                    process_id: 0,
                    process_name: "synthetic".into(),
                    window_title: String::new(),
                },
                held_worker,
                WeakEntity::new_invalid(),
                uuid::Uuid::new_v4(),
                None,
                window,
                cx,
            )
        });
        *retained.borrow_mut() = Some(panel.clone());
        popup_surface::root(panel, window, cx)
    });
    surface.simulate_resize(size(px(340.0), px(440.0)));
    surface.refresh().expect("actual ClipboardPanel");
    let panel = slot.borrow().clone().expect("retained panel");
    let root_id = root.entity_id();
    let view_id = panel.entity_id();
    let input_id = panel.read_with(surface, |panel, _| panel.search_input.entity_id());
    panel.update_in(surface, |panel, window, cx| {
        panel.focus_search_input(window, cx)
    });
    let focus = surface.update(|window, cx| window.focused(cx));
    let selection = panel.read_with(surface, |panel, _| panel.selected_entry_index);
    for preference in [ThemePreference::Dark, ThemePreference::Light] {
        panel.update_in(surface, |_, _, cx| {
            crate::theme::set_appearance_preferences(preference, MaterialStyle::Acrylic, cx);
        });
        surface.run_until_parked();
        // No current receipt: opaque foundation.
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
        assert_eq!(
            panel.read_with(surface, |panel, _| panel.selected_entry_index),
            selection
        );
        assert_eq!(surface.update(|window, cx| window.focused(cx)), focus);
    }
    surface.simulate_resize(size(px(280.0), px(280.0)));
    assert_exterior(surface, None);
    worker.shutdown().expect("synthetic worker shutdown");
}

#[gpui_kit::test]
fn corner_surface_plain_root_stays_square(cx: &mut TestAppContext) {
    init(cx);
    struct Content;
    impl Render for Content {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Content);
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(340.0), px(440.0)));
    surface.refresh().expect("ordinary Root control");
    surface.update(|window, _| {
        assert!(
            window
                .painted_quads()
                .iter()
                .filter(|quad| !quad.background.is_transparent())
                .all(|quad| quad.corner_radii == Corners::default())
        );
    });
}

#[gpui_kit::test]
fn corner_surface_tiled_maximized_and_fullscreen_policy(_cx: &mut TestAppContext) {
    use popup_surface::corner_radii;
    assert_eq!(
        corner_radii(Decorations::Server, false, false, px(8.0)),
        Corners::all(px(8.0))
    );
    // Windows 10 reports no system rounding: the popup stays square.
    assert_eq!(
        corner_radii(Decorations::Server, false, false, px(0.0)),
        Corners::default()
    );
    for decorations in [
        Decorations::Server,
        Decorations::Client {
            tiling: Tiling::default(),
        },
    ] {
        assert_eq!(
            corner_radii(decorations, true, false, px(8.0)),
            Corners::default()
        );
        assert_eq!(
            corner_radii(decorations, false, true, px(8.0)),
            Corners::default()
        );
    }
    assert_eq!(
        corner_radii(
            Decorations::Client {
                tiling: Tiling {
                    left: true,
                    ..Default::default()
                }
            },
            false,
            false,
            px(8.0)
        ),
        Corners {
            top_left: px(0.0),
            bottom_left: px(0.0),
            top_right: px(8.0),
            bottom_right: px(8.0)
        }
    );
    assert_eq!(
        corner_radii(
            Decorations::Client {
                tiling: Tiling {
                    top: true,
                    bottom: true,
                    left: true,
                    right: true
                }
            },
            false,
            false,
            px(8.0)
        ),
        Corners::default()
    );
}
