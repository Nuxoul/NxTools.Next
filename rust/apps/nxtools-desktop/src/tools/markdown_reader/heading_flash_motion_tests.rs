use std::cell::RefCell;

use gpui_kit::{TestAppContext, VisualTestContext};

use super::*;
use crate::ui::motion::UiMotionPreference;

fn settle(surface: &mut VisualTestContext) {
    for _ in 0..3 {
        surface.refresh().expect("render reader motion fixture");
        surface.run_until_parked();
    }
}

fn fixture(cx: &mut TestAppContext) -> (Entity<Reader>, &mut VisualTestContext, tempfile::TempDir) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.update(|cx| {
        cx.set_reduce_motion(true);
        cx.set_global(UiMotionPreference::Known {
            reduced_motion: true,
        });
    });
    let slot = Rc::new(RefCell::new(None));
    let reader_slot = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let reader = cx.new(|cx| Reader::new(None, window, cx));
        *reader_slot.borrow_mut() = Some(reader.clone());
        gpui_kit::base::Root::new(reader, window, cx)
    });
    let reader = slot.borrow().clone().expect("reader under production Root");
    let directory = tempfile::tempdir().expect("heading motion fixture directory");
    let path = directory.path().join("heading-motion.md");
    let paragraph = "Synthetic reader paragraph for heading navigation. ".repeat(100);
    std::fs::write(
        &path,
        format!("# Start\n\n{paragraph}\n\n## Target\n\n{paragraph}\n\n## End\n\n{paragraph}\n"),
    )
    .expect("write synthetic heading fixture");
    let document = document::read(path).expect("read synthetic heading fixture");
    surface.simulate_resize(size(px(1020.), px(720.)));
    surface.update(|window, cx| {
        window.blur(cx);
        reader.update(cx, |reader, cx| assert!(reader.add_document(document, cx)));
    });
    settle(surface);
    (reader, surface, directory)
}

fn policy(surface: &mut VisualTestContext, preference: UiMotionPreference) {
    surface.update(|_, cx| {
        cx.set_global(preference);
    });
    surface.run_until_parked();
}

fn jump(reader: &Entity<Reader>, surface: &mut VisualTestContext, heading: usize) {
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            let tab = reader.active_tab().expect("open tab");
            let id = tab.id;
            let anchor = tab.navigation.headings(&tab.document)[heading]
                .anchor
                .clone();
            reader.jump_outline(id, &anchor, window, cx);
        });
    });
}

fn paint(surface: &mut VisualTestContext) {
    surface.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.refresh();
        window.draw(cx).clear(cx);
    });
    surface.run_until_parked();
}

// Inspect the actual canvas quad at the committed heading's visible geometry.
fn flash_alpha(reader: &Entity<Reader>, surface: &mut VisualTestContext) -> Option<f32> {
    let (bounds, accent) = surface.read(|cx| {
        let view = reader
            .read(cx)
            .active_tab()
            .expect("open tab")
            .view
            .read(cx);
        let list = view.list_state();
        (
            list.bounds_for_item(list.logical_scroll_top().item_ix)
                .expect("visible target block")
                .intersect(&list.viewport_bounds()),
            gpui_kit::Hsla::from(Theme::active(cx).accent),
        )
    });
    surface.update(|window, _| {
        let scale = window.scale_factor();
        window.painted_quads().iter().find_map(|quad| {
            let color = quad.background.as_solid()?;
            let same_bounds = [
                (
                    quad.bounds.origin.x.as_f32() / scale,
                    bounds.left().as_f32(),
                ),
                (quad.bounds.origin.y.as_f32() / scale, bounds.top().as_f32()),
                (
                    quad.bounds.size.width.as_f32() / scale,
                    bounds.size.width.as_f32(),
                ),
                (
                    quad.bounds.size.height.as_f32() / scale,
                    bounds.size.height.as_f32(),
                ),
            ]
            .into_iter()
            .all(|(actual, expected)| (actual - expected).abs() < 0.1);
            (same_bounds && (color.h, color.s, color.l) == (accent.h, accent.s, accent.l))
                .then_some(color.a / accent.a)
        })
    })
}

fn queued_frames(surface: &mut VisualTestContext) -> usize {
    surface.update(|window, cx| window.simulate_next_frame(cx))
}

#[gpui_kit::test]
fn heading_flash_motion_reduced_and_unknown_navigate_without_frames(cx: &mut TestAppContext) {
    let (reader, surface, _directory) = fixture(cx);
    for mode in [edit::Mode::Preview, edit::Mode::Split] {
        surface.update(|window, cx| {
            reader.update(cx, |reader, cx| reader.set_mode(mode, window, cx));
            window.blur(cx);
        });
        settle(surface);
        for preference in [
            UiMotionPreference::Known {
                reduced_motion: true,
            },
            UiMotionPreference::Unknown,
        ] {
            policy(surface, preference);
            let (view, revision) = surface.read(|cx| {
                let tab = reader.read(cx).active_tab().expect("tab");
                (tab.view.clone(), tab.view.read(cx).rendered_text())
            });
            jump(&reader, surface, 1);
            settle(surface);
            paint(surface);
            assert!(flash_alpha(&reader, surface).is_none());
            assert_eq!(queued_frames(surface), 0, "no decorative frame loop");
            surface.read(|cx| {
                let tab = reader.read(cx).active_tab().expect("same tab");
                assert_eq!(tab.view.entity_id(), view.entity_id());
                assert_eq!(tab.view.read(cx).rendered_text(), revision);
                assert_eq!(tab.navigation.active_heading, Some(1));
                assert!(tab.view.read(cx).list_state().logical_scroll_top().item_ix > 0);
            });
            policy(
                surface,
                UiMotionPreference::Known {
                    reduced_motion: false,
                },
            );
            paint(surface);
            assert!(
                flash_alpha(&reader, surface).is_none(),
                "no replay on opt-in"
            );
        }
    }
}

#[gpui_kit::test]
fn heading_flash_motion_normal_fades_once_and_stops_at_1200ms(cx: &mut TestAppContext) {
    let (reader, surface, _directory) = fixture(cx);
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: false,
        },
    );
    jump(&reader, surface, 1);
    paint(surface);
    let initial = flash_alpha(&reader, surface).expect("normal target feedback");
    assert!((initial - 0.14).abs() < 0.001);
    assert!(queued_frames(surface) > 0, "finite fade requests a frame");
    surface.executor().advance_clock(Duration::from_millis(600));
    paint(surface);
    let halfway = flash_alpha(&reader, surface).expect("halfway feedback");
    assert!((halfway - 0.07).abs() < 0.001);
    paint(surface);
    assert_eq!(
        flash_alpha(&reader, surface),
        Some(halfway),
        "repaint does not rearm"
    );
    surface.executor().advance_clock(Duration::from_millis(600));
    paint(surface);
    assert!(flash_alpha(&reader, surface).is_none());
    assert_eq!(
        queued_frames(surface),
        0,
        "settled reader has no flash clock"
    );
    paint(surface);
    assert!(flash_alpha(&reader, surface).is_none());
}

#[gpui_kit::test]
fn heading_flash_motion_preference_change_retires_inflight_flash(cx: &mut TestAppContext) {
    let (reader, surface, _directory) = fixture(cx);
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: false,
        },
    );
    jump(&reader, surface, 1);
    paint(surface);
    assert!(flash_alpha(&reader, surface).is_some());
    surface.executor().advance_clock(Duration::from_millis(100));
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: true,
        },
    );
    paint(surface);
    assert!(flash_alpha(&reader, surface).is_none());
    assert_eq!(queued_frames(surface), 0);
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: false,
        },
    );
    paint(surface);
    assert!(
        flash_alpha(&reader, surface).is_none(),
        "obsolete flash cannot resume"
    );
    jump(&reader, surface, 2);
    paint(surface);
    assert!(
        flash_alpha(&reader, surface).is_some(),
        "fresh navigation may flash"
    );
}

#[gpui_kit::test]
fn heading_flash_motion_preference_change_retires_background_tab_flash(cx: &mut TestAppContext) {
    let (reader, surface, directory) = fixture(cx);
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: false,
        },
    );
    jump(&reader, surface, 1);
    paint(surface);
    assert!(flash_alpha(&reader, surface).is_some());
    let path = directory.path().join("other.md");
    std::fs::write(&path, "# Other\n\nSynthetic other tab.\n").expect("write second fixture");
    let document = document::read(path).expect("read second fixture");
    surface.update(|_, cx| {
        reader.update(cx, |reader, cx| assert!(reader.add_document(document, cx)));
    });
    settle(surface);
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: true,
        },
    );
    policy(
        surface,
        UiMotionPreference::Known {
            reduced_motion: false,
        },
    );
    surface.update(|_, cx| reader.update(cx, |reader, cx| reader.activate(0, cx)));
    paint(surface);
    assert!(
        flash_alpha(&reader, surface).is_none(),
        "hidden obsolete flash cannot resume"
    );
}
