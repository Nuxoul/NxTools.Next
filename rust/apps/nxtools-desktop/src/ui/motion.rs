use std::time::Duration;

use gpui::{App, Global};
use nxtools_protocol::SystemMotionPreferenceChanged;

/// Unknown native preference is deliberately treated as reduced motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum UiMotionPreference {
    #[default]
    Unknown,
    Known {
        reduced_motion: bool,
    },
}

impl Global for UiMotionPreference {}

impl UiMotionPreference {
    pub(crate) fn apply(&mut self, event: &SystemMotionPreferenceChanged) -> bool {
        let next = Self::Known {
            reduced_motion: event.reduced_motion || event.native_error_code != 0,
        };
        if *self == next {
            return false;
        }
        *self = next;
        true
    }

    pub(crate) fn reduced_motion(self) -> bool {
        !matches!(
            self,
            Self::Known {
                reduced_motion: false
            }
        )
    }
}

pub(crate) fn reduced_motion(cx: &App) -> bool {
    cx.try_global::<UiMotionPreference>()
        .copied()
        .unwrap_or_default()
        .reduced_motion()
}

pub(crate) fn motion_allowed(cx: &App) -> bool {
    !reduced_motion(cx)
}

pub(crate) fn observe_motion<T: 'static>(window: &mut gpui::Window, cx: &mut gpui::Context<T>) {
    let view = cx.entity().downgrade();
    let id = gpui::SharedString::from(format!("ui-motion-observer-{:?}", cx.entity().entity_id()));
    let _ = window.use_keyed_state(id, cx, move |_, cx| {
        cx.observe_global::<UiMotionPreference>(move |_, cx| {
            let view = view.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |_, cx| cx.notify());
            });
        })
    });
}

pub(crate) const ENTER_DURATION_MS: u64 = 120;
#[allow(dead_code)] // Applied as exit surfaces migrate to the shared layer.
pub(crate) const EXIT_DURATION_MS: u64 = 120;
#[allow(dead_code)] // Applied as resizing surfaces migrate to the shared layer.
pub(crate) const RESIZE_DURATION_MS: u64 = 160;

pub(crate) const ENTER_DURATION: Duration = Duration::from_millis(ENTER_DURATION_MS);

/// Read current policy at the render boundary; do not create zero-duration frames.
pub(crate) fn animated_surface(
    id: impl Into<gpui::ElementId>,
    duration: Duration,
    content: gpui::AnyElement,
    offset: f32,
    initial_opacity: f32,
) -> gpui::AnyElement {
    use gpui::IntoElement;
    MotionSurface {
        id: id.into(),
        duration,
        content,
        offset,
        initial_opacity,
    }
    .into_any_element()
}

#[derive(gpui::IntoElement)]
struct MotionSurface {
    id: gpui::ElementId,
    duration: Duration,
    content: gpui::AnyElement,
    offset: f32,
    initial_opacity: f32,
}
impl gpui::RenderOnce for MotionSurface {
    fn render(self, _: &mut gpui::Window, cx: &mut App) -> impl gpui::IntoElement {
        use gpui::{AnimationExt, IntoElement, prelude::*};
        let Self {
            id,
            duration,
            content,
            offset,
            initial_opacity,
        } = self;
        let surface = gpui::div().size_full().child(content);
        if motion_allowed(cx) {
            surface
                .with_animation(
                    id,
                    gpui::Animation::new(duration).with_easing(gpui::ease_out_quint()),
                    move |surface, delta| {
                        surface
                            .opacity(initial_opacity + (1.0 - initial_opacity) * delta)
                            .relative()
                            .top(gpui::px(offset * (1.0 - delta)))
                    },
                )
                .into_any_element()
        } else {
            surface.into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn reduced_motion_surface_renders_final_geometry_without_spatial_frames(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::{IntoElement, prelude::*};
        struct MotionFixture;
        impl gpui::Render for MotionFixture {
            fn render(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                observe_motion(window, cx);
                gpui::div().size_full().child(animated_surface(
                    "motion-regression",
                    ENTER_DURATION,
                    gpui::div()
                        .debug_selector(|| "motion-regression-content".into())
                        .size_full()
                        .into_any_element(),
                    12.0,
                    0.5,
                ))
            }
        }
        let (_, window) = cx.add_window_view(|window, _| {
            window.resize(gpui::size(gpui::px(200.0), gpui::px(100.0)));
            MotionFixture
        });
        window.simulate_resize(gpui::size(gpui::px(200.0), gpui::px(100.0)));
        window.update(|_, cx| {
            cx.set_global(UiMotionPreference::Known {
                reduced_motion: true,
            })
        });
        window.refresh().expect("paint reduced-motion final frame");
        let reduced = window
            .debug_bounds("motion-regression-content")
            .expect("final content bounds");
        assert_eq!(reduced.origin.y, gpui::px(0.0));
        assert_eq!(reduced.size, gpui::size(gpui::px(200.0), gpui::px(100.0)));
        window.update(|_, cx| {
            cx.set_global(UiMotionPreference::Known {
                reduced_motion: false,
            })
        });
        window.run_until_parked();
        window.refresh().expect("paint permitted entrance");
        let allowed = window
            .debug_bounds("motion-regression-content")
            .expect("animated content bounds");
        assert!(allowed.origin.y > reduced.origin.y);
        window.update(|_, cx| {
            cx.set_global(UiMotionPreference::Known {
                reduced_motion: true,
            });
            cx.refresh_windows();
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("system preference settles existing entrance immediately");
        assert_eq!(
            window.debug_bounds("motion-regression-content"),
            Some(reduced)
        );
    }

    #[test]
    fn motion_tokens_remain_short_and_bounded() {
        assert!((100..=140).contains(&ENTER_DURATION_MS));
        assert!((100..=140).contains(&EXIT_DURATION_MS));
        assert!((140..=180).contains(&RESIZE_DURATION_MS));
        assert_eq!(ENTER_DURATION, Duration::from_millis(120));
    }

    #[gpui::test]
    fn native_motion_policy_defaults_safe_and_tracks_transitions(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            assert!(reduced_motion(cx));
            assert!(!motion_allowed(cx));
            let mut preference = UiMotionPreference::default();
            let mut event = SystemMotionPreferenceChanged {
                correlation_id: uuid::Uuid::new_v4(),
                reduced_motion: false,
                native_error_code: 0,
            };
            assert!(preference.apply(&event));
            assert!(!preference.apply(&event));
            cx.set_global(preference);
            assert!(motion_allowed(cx));
            event.reduced_motion = true;
            assert!(preference.apply(&event));
            cx.set_global(preference);
            assert!(reduced_motion(cx));
            event.reduced_motion = false;
            event.native_error_code = 5;
            assert!(!preference.apply(&event));
            assert!(reduced_motion(cx));
            event.native_error_code = 0;
            assert!(preference.apply(&event));
            cx.set_global(preference);
            assert!(motion_allowed(cx));
        });
    }
}
