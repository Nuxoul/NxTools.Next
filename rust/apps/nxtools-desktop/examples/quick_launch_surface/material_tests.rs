use super::*;
use gpui_kit::{ElementId, Modifiers, TestAppContext, VisualTestContext, point};
use std::{cell::RefCell, rc::Rc};

fn fixture(
    cx: &mut TestAppContext,
    surface: Surface,
    material: Material,
) -> (Entity<MaterialSurface>, &mut VisualTestContext) {
    cx.update(|cx| super::super::initialize(cx, true));
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        window.set_background_appearance(WindowBackgroundAppearance::Transparent);
        let view = cx.new(|cx| MaterialSurface::new(surface, material, true, window, cx));
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    window.simulate_resize(surface.dimensions());
    window.run_until_parked();
    window.refresh().expect("actual material fixture frame");
    (retained.borrow_mut().take().expect("material view"), window)
}

#[gpui_kit::test]
fn real_material_frames_keep_the_requested_fill_and_input_bounds(cx: &mut TestAppContext) {
    for surface in [Surface::Workbench, Surface::Clipboard, Surface::Search] {
        for material in [Material::Solid, Material::Acrylic] {
            let (_, window) = fixture(cx, surface, material);
            let dimensions = surface.dimensions();
            let expected_alpha = if material == Material::Acrylic {
                0.78
            } else {
                1.0
            };
            let (quads, scale, expected) = window.update(|window, cx| {
                let color = if surface == Surface::Search {
                    cx.theme().popover
                } else {
                    cx.theme().background
                }
                .opacity(expected_alpha);
                (
                    window.painted_quads(),
                    window.scale_factor(),
                    gpui_kit::solid_background(color),
                )
            });
            assert!(
                quads.iter().any(|quad| {
                    quad.background == expected
                        && (quad.bounds.size.width.as_f32() / scale - dimensions.width.as_f32())
                            .abs()
                            < 0.1
                        && (quad.bounds.size.height.as_f32() / scale - dimensions.height.as_f32())
                            .abs()
                            < 0.1
                        && quad.corner_radii.top_left.as_f32() / scale == 8.0
                }),
                "the actual rendered outer contour must carry the requested material alpha"
            );
            let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
            let input = nodes
                .iter()
                .find(|node| node.label() == Some("原生主题输入"))
                .expect("official Input")
                .bounds();
            assert!(input.left() >= px(0.0) && input.top() >= px(0.0));
            assert!(input.right() <= dimensions.width && input.bottom() <= dimensions.height);
            if surface == Surface::Search {
                let group = nodes
                    .iter()
                    .find(|node| node.path().last() == Some(&ElementId::from("native-search")))
                    .expect("official InputGroup")
                    .bounds();
                assert_eq!(group, Bounds::new(point(px(0.0), px(0.0)), dimensions));
            }
        }
    }
}

#[gpui_kit::test]
fn native_material_selection_and_theme_preserve_input_and_close_owner(cx: &mut TestAppContext) {
    let (view, window) = fixture(cx, Surface::Clipboard, Material::Acrylic);
    let input = view.read_with(window, |view, _| view.input.clone());
    let identity = input.entity_id();
    window.simulate_input("中文 GPUI 123");
    window.run_until_parked();
    let selected = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.path().last() == Some(&ElementId::from("native-record-1")))
        .expect("second released ListItem")
        .bounds();
    window.simulate_click(selected.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(view.read_with(window, |view, _| view.selected), 1);
    window.simulate_keystrokes("f2");
    window.run_until_parked();
    window.refresh().expect("deferred light material frame");
    assert!(!view.read_with(window, |view, _| view.dark));
    assert_eq!(
        view.read_with(window, |view, _| view.input.entity_id()),
        identity
    );
    assert_eq!(
        window.read(|cx| input.read(cx).value().to_string()),
        "中文 GPUI 123"
    );
    window.simulate_keystrokes("escape");
    window.run_until_parked();
    assert!(window.read(|cx| cx.windows().is_empty()));
}

#[::core::prelude::v1::test]
fn native_material_arguments_do_not_silently_select_another_surface() {
    let args = ["--surface=clipboard", "--material=acrylic"].map(str::to_owned);
    let options = Options::parse(&args).expect("known native fixture options");
    assert_eq!(options.surface, Surface::Clipboard);
    assert_eq!(options.material, Material::Acrylic);
    assert!(Options::parse(&["--surface=unknown".into()]).is_err());
    assert!(Options::parse(&["--material=unknown".into()]).is_err());
    assert!(Options::parse(&["--gpu-capture=unused".into()]).is_err());
}

#[gpui_kit::test]
fn benchmark_input_and_virtual_scroll_run_and_stop_without_replacing_state(
    cx: &mut TestAppContext,
) {
    use super::benchmark::{BenchmarkState, Phase};
    use std::time::Duration;
    let (view, window) = fixture(cx, Surface::Workbench, Material::Acrylic);
    let input = view.read_with(window, |view, _| view.input.clone());
    let identity = input.entity_id();
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.benchmark = Some(BenchmarkState::new());
            view.set_benchmark_phase(Phase::Input, window, cx);
        })
    });
    window.run_until_parked();
    for _ in 0..10 {
        window.executor().advance_clock(Duration::from_millis(50));
        window.run_until_parked();
    }
    let updates = view.read_with(window, |view, _| view.benchmark.as_ref().unwrap().updates);
    assert!(updates >= 8, "actual bounded InputState work ran");
    assert_eq!(input.entity_id(), identity);
    assert_eq!(window.read(|cx| input.read(cx).value().len()), 16);
    window.update(|_, cx| view.update(cx, |view, _| view.stop_benchmark_phase()));
    window.run_until_parked();
    let value = window.read(|cx| input.read(cx).value());
    window.executor().advance_clock(Duration::from_secs(1));
    window.run_until_parked();
    assert_eq!(window.read(|cx| input.read(cx).value()), value);
    assert_eq!(
        view.read_with(window, |view, _| view.benchmark.as_ref().unwrap().updates),
        updates
    );
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.set_benchmark_phase(Phase::Scroll, window, cx)
        })
    });
    window.run_until_parked();
    for _ in 0..30 {
        window.executor().advance_clock(Duration::from_millis(16));
        window.run_until_parked();
        window
            .refresh()
            .expect("actual virtualized benchmark frame");
    }
    assert!(
        view.read_with(window, |view, _| view
            .benchmark
            .as_ref()
            .unwrap()
            .max_rendered_row
            .get())
            >= 20,
        "uniform_list must actually render later records, not merely receive scroll requests"
    );
    window.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.stop_benchmark_phase();
            view.set_benchmark_phase(Phase::Idle, window, cx);
        })
    });
    window.run_until_parked();
    window.executor().advance_clock(Duration::from_secs(1));
    window.run_until_parked();
    assert_eq!(
        view.read_with(window, |view, _| view.benchmark.as_ref().unwrap().updates),
        0
    );
}

#[::core::prelude::v1::test]
fn benchmark_flags_are_explicit_and_bounded() {
    assert!(Options::parse(&[]).unwrap().benchmark.is_none());
    for flag in [
        "--benchmark=unknown",
        "--benchmark-seconds=0",
        "--benchmark-seconds=121",
    ] {
        assert!(Options::parse(&[flag.into()]).is_err());
    }
    assert_eq!(
        Options::parse(&["--benchmark=multiwindow".into()])
            .unwrap()
            .benchmark,
        Some(super::benchmark::Scenario::Multiwindow)
    );
}
