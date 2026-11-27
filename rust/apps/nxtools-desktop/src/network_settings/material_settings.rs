use gpui_kit::component::select::{SearchableVec, Select, SelectItem, SelectState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::{IndexPath, Sizable, Size};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, IntoElement, SharedString, Window, div,
    prelude::*, px,
};
use nxtools_core::{MaterialStyle, MaterialTuning};

#[derive(Clone)]
struct MaterialChoice {
    style: MaterialStyle,
    label: &'static str,
}

impl SelectItem for MaterialChoice {
    type Value = MaterialStyle;

    fn title(&self) -> SharedString {
        self.label.into()
    }

    fn value(&self) -> &MaterialStyle {
        &self.style
    }
}

/// Original window-local draft; only the parent Save publishes durable intent.
pub(super) struct MaterialSettingsDraft {
    selector: Entity<SelectState<SearchableVec<MaterialChoice>>>,
    content: Entity<SliderState>,
}

impl MaterialSettingsDraft {
    pub(super) fn new<T: 'static>(
        style: MaterialStyle,
        tuning: MaterialTuning,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> Self {
        // Windows 10 names its host-backdrop frosting "磨砂".
        let system_backdrop = crate::ui::material::system_backdrop(cx);
        let choices: Vec<MaterialChoice> = MaterialStyle::ALL
            .iter()
            .map(|&style| MaterialChoice {
                style,
                label: style.label_for(system_backdrop),
            })
            .collect();
        let selected = choices.iter().position(|choice| choice.style == style);
        let content = cx.new(|_| {
            SliderState::new()
                .min(f32::from(MaterialTuning::MIN_PERCENT))
                .max(f32::from(MaterialTuning::MAX_PERCENT))
                .step(1.0)
                .default_value(f32::from(tuning.content_percent))
        });
        cx.subscribe(&content, |_, _, _: &SliderEvent, cx| cx.notify())
            .detach();
        Self {
            selector: cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(choices),
                    selected.map(|row| IndexPath::default().row(row)),
                    window,
                    cx,
                )
            }),
            content,
        }
    }

    pub(super) fn selected(&self, cx: &App) -> MaterialStyle {
        self.selector
            .read(cx)
            .selected_value()
            .copied()
            .expect("material choice is always selected")
    }

    pub(super) fn tuning(&self, cx: &App) -> MaterialTuning {
        MaterialTuning {
            content_percent: percent(&self.content, cx),
        }
    }

    pub(super) fn render(&self, saving: bool) -> AnyElement {
        Select::new(&self.selector)
            .id("global-material-style")
            .accessibility_label("主题风格")
            .with_size(Size::Medium)
            .w_full()
            .disabled(saving)
            .into_any_element()
    }

    pub(super) fn render_content(&self, saving: bool, cx: &App) -> AnyElement {
        slider_row(&self.content, "内容层不透明度", saving, cx)
    }
}

fn percent(state: &Entity<SliderState>, cx: &App) -> u8 {
    state.read(cx).value().start().round().clamp(
        f32::from(MaterialTuning::MIN_PERCENT),
        f32::from(MaterialTuning::MAX_PERCENT),
    ) as u8
}

fn slider_row(
    state: &Entity<SliderState>,
    label: &'static str,
    saving: bool,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .w_full()
        .child(
            Slider::new(state)
                .accessibility_label(label)
                .disabled(saving)
                .flex_1(),
        )
        .child(
            // Room for "100%" at any UI font, never wrapped.
            div()
                .min_w(px(56.0))
                .flex_none()
                .whitespace_nowrap()
                .text_right()
                .child(format!("{}%", percent(state, cx))),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_settings::kit_migration_tests::{fixture, select_page};
    use gpui_kit::{TestAppContext, VisualTestContext};
    use nxtools_core::{AppConfig, ThemePreference};

    fn choose(surface: &mut VisualTestContext, style: MaterialStyle) {
        let bounds = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&"global-material-style".into()))
                .expect("official Select")
                .bounds()
        });
        assert_eq!(bounds.size.height, gpui_kit::px(32.0));
        surface.simulate_click(bounds.center(), Default::default());
        surface.run_until_parked();
        surface.refresh().expect("official dropdown");
        surface.simulate_keystrokes(match style {
            MaterialStyle::Solid => "up enter",
            MaterialStyle::Acrylic => "down enter",
        });
        surface.run_until_parked();
        surface.refresh().expect("selected draft");
    }

    #[gpui_kit::test]
    fn material_style_real_selector_cancel_save_failure_and_no_shell_reconfiguration(
        cx: &mut TestAppContext,
    ) {
        let (app, view, mut surface, root) = fixture(cx);
        let path = root.join("config.json");
        let original = std::fs::read(&path).expect("original");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let platform = std::rc::Rc::new(bridge);
        app.update(&mut surface, |app, _| app.platform = Some(platform.clone()));
        surface.update(|_, cx| {
            use nxtools_protocol::window_material::*;
            let mut state = crate::ui::material::UiMaterialCapabilities::default();
            state.connected();
            assert!(state.apply(SystemMaterialCapabilitiesChanged {
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
            cx.set_global(state);
            cx.observe_new::<gpui_kit::base::Root>(move |root, window, cx| {
                if root
                    .view()
                    .clone()
                    .downcast::<crate::network_settings::NetworkSettingsView>()
                    .is_err()
                {
                    return;
                }
                let Some(window) = window else {
                    return;
                };
                let handle = window.window_handle();
                crate::ui::material::runtime::set_test_window_handle(handle, 6350, cx);
                let command = crate::ui::material::command(6350, MaterialSurface::Backdrop, cx)
                    .expect("facts");
                let applied = AppliedWindowMaterial {
                    session_id: command.session_id,
                    facts_revision: command.facts_revision,
                    window_handle: command.window_handle,
                    generation: command.generation,
                    style: command.style,
                    surface: command.surface,
                    dark: command.dark,
                    effective: if command.style == WindowMaterialStyle::Solid {
                        EffectiveWindowMaterial::Solid
                    } else {
                        EffectiveWindowMaterial::Acrylic
                    },
                    fallback_reason: MaterialFallbackReason::None,
                    native_error_code: 0,
                };
                let prepared = crate::ui::material::Preparation::confirmed(
                    uuid::Uuid::new_v4(),
                    command,
                    applied,
                    cx,
                )
                .expect("coherent fixture");
                assert!(crate::ui::material::runtime::track_prepared(
                    handle,
                    platform.clone(),
                    prepared,
                    cx
                ));
            })
            .detach();
        });
        select_page(&mut surface, 3);
        choose(&mut surface, MaterialStyle::Acrylic);
        assert_eq!(
            view.read_with(&surface, |view, cx| view.material.selected(cx)),
            MaterialStyle::Acrylic
        );
        assert_eq!(std::fs::read(&path).expect("draft not saved"), original);
        assert_eq!(
            surface.read(|cx| cx.global::<crate::theme::UiThemeState>().material_style),
            MaterialStyle::Solid
        );
        view.update(&mut surface, |view, cx| {
            view.saving = true;
            cx.notify();
        });
        surface.refresh().expect("busy selector");
        let disabled = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&"global-material-style".into()))
                .expect("disabled official Select")
                .bounds()
        });
        surface.simulate_click(disabled.center(), Default::default());
        surface.run_until_parked();
        assert_eq!(
            view.read_with(&surface, |view, cx| view.material.selected(cx)),
            MaterialStyle::Acrylic
        );
        view.update(&mut surface, |view, cx| {
            view.saving = false;
            cx.notify();
        });
        surface.refresh().expect("retryable draft");
        let cancel = surface
            .debug_bounds("network-settings-cancel")
            .expect("Cancel");
        surface.simulate_click(cancel.center(), Default::default());
        surface.run_until_parked();
        assert_eq!(std::fs::read(&path).expect("Cancel"), original);
        app.update(&mut surface, |app, cx| app.open_window(cx));
        surface.run_until_parked();
        let (handle, view) = app.read_with(&surface, |app, _| {
            (
                app.window.expect("reopen"),
                app.settings_view.clone().expect("draft"),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(gpui_kit::size(gpui_kit::px(720.0), gpui_kit::px(480.0)));
        assert_eq!(
            view.read_with(&surface, |view, cx| view.material.selected(cx)),
            MaterialStyle::Solid
        );
        select_page(&mut surface, 3);
        choose(&mut surface, MaterialStyle::Acrylic);
        let save = surface
            .debug_bounds("network-settings-save")
            .expect("Apply");
        surface.simulate_click(save.center(), Default::default());
        surface.run_until_parked();
        let saved = AppConfig::load_from(&path).expect("saved");
        assert_eq!(saved.system.material_style, MaterialStyle::Acrylic);
        assert_eq!(saved.system.theme, ThemePreference::System);
        assert_eq!(
            saved.system.ui_font_family,
            nxtools_core::DEFAULT_UI_FONT_FAMILY
        );
        assert_eq!(
            surface.read(|cx| cx.global::<crate::theme::UiThemeState>().material_style),
            MaterialStyle::Acrylic
        );
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "style-only save must not reconfigure shell"
        );
        app.update(&mut surface, |app, cx| app.open_window(cx));
        surface.run_until_parked();
        let (handle, view) = app.read_with(&surface, |app, _| {
            (
                app.window.expect("retry"),
                app.settings_view.clone().expect("draft"),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(gpui_kit::size(gpui_kit::px(720.0), gpui_kit::px(480.0)));
        select_page(&mut surface, 3);
        choose(&mut surface, MaterialStyle::Solid);
        std::fs::write(&path, b"{broken").expect("owned persistence failure");
        let save = surface
            .debug_bounds("network-settings-save")
            .expect("Apply");
        surface.simulate_click(save.center(), Default::default());
        surface.run_until_parked();
        view.read_with(&surface, |view, cx| {
            assert!(!view.saving);
            assert_eq!(view.material.selected(cx), MaterialStyle::Solid);
        });
        assert_eq!(
            surface.read(|cx| cx.global::<crate::theme::UiThemeState>().material_style),
            MaterialStyle::Acrylic
        );
        assert_eq!(std::fs::read(&path).expect("failure bytes"), b"{broken");
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        app.update(&mut surface, |app, cx| app.close_window(cx));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[gpui_kit::test]
    fn material_style_real_save_publishes_disk_merge_only_after_deferred_commit(
        cx: &mut TestAppContext,
    ) {
        let (app, view, mut surface, root) = fixture(cx);
        let path = root.join("config.json");
        let family = crate::ui::typography::TEST_UI_FONT_FAMILY;
        surface.update(|_, cx| {
            cx.set_global(crate::ui::typography::InstalledUiFontsFixture(vec![
                family.into(),
            ]))
        });
        let mut latest = AppConfig::load_from(&path).expect("baseline");
        latest.system.theme = ThemePreference::GruvboxLight;
        latest.system.ui_font_family = family.into();
        latest.system.notification_duration_seconds = 19;
        latest.save_to(&path).expect("concurrent saved settings");
        view.update_in(&mut surface, |view, window, cx| {
            view.material.selector.update(cx, |state, cx| {
                state.set_selected_value(&MaterialStyle::Acrylic, window, cx)
            });
            view.save(window, cx);
            assert_eq!(
                cx.global::<crate::theme::UiThemeState>().material_style,
                MaterialStyle::Solid
            );
            assert_eq!(
                AppConfig::load_from(&path)
                    .expect("before deferred commit")
                    .system
                    .material_style,
                MaterialStyle::Solid
            );
        });
        surface.run_until_parked();
        let saved = AppConfig::load_from(&path).expect("merged");
        assert_eq!(saved.system.material_style, MaterialStyle::Acrylic);
        assert_eq!(saved.system.theme, ThemePreference::GruvboxLight);
        assert_eq!(saved.system.ui_font_family, family);
        surface.read(|cx| {
            let state = cx.global::<crate::theme::UiThemeState>();
            assert_eq!(state.material_style, MaterialStyle::Acrylic);
            assert_eq!(state.preference, ThemePreference::GruvboxLight);
            assert_eq!(crate::ui::typography::active_ui_font_family(cx), family);
        });
        app.read_with(&surface, |app, cx| {
            assert_eq!(app.config.system, saved.system);
            assert_eq!(*app.system_settings.borrow(), saved.system);
            assert_eq!(app.notifications.read(cx).duration_seconds(), 19);
        });
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[gpui_kit::test]
    fn material_content_slider_saves_and_publishes_without_touching_style_or_palette(
        cx: &mut TestAppContext,
    ) {
        let (_app, view, mut surface, root) = fixture(cx);
        let path = root.join("config.json");
        let original = AppConfig::load_from(&path).expect("baseline");
        assert_eq!(original.system.material_tuning, MaterialTuning::default());
        view.update_in(&mut surface, |view, window, cx| {
            assert_eq!(view.material.tuning(cx), MaterialTuning::default());
            view.material
                .content
                .update(cx, |state, cx| state.set_value(65.0, window, cx));
            assert_eq!(
                AppConfig::load_from(&path)
                    .expect("draft is not saved")
                    .system,
                original.system
            );
            view.save(window, cx);
        });
        surface.run_until_parked();
        let tuning = MaterialTuning {
            content_percent: 65,
        };
        let saved = AppConfig::load_from(&path).expect("saved");
        assert_eq!(saved.system.material_tuning, tuning);
        assert_eq!(saved.system.material_style, original.system.material_style);
        assert_eq!(saved.system.theme, original.system.theme);
        assert_eq!(saved.system.ui_font_family, original.system.ui_font_family);
        surface.read(|cx| {
            let state = cx.global::<crate::theme::UiThemeState>();
            assert_eq!(state.tuning, tuning);
            assert_eq!(state.material_style, original.system.material_style);
        });
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    /// Composite coverage of the painted solid fills, sampled every 12 DIP.
    fn coverage(surface: &mut VisualTestContext) -> (Vec<String>, f32) {
        surface.update(|window, _| {
            let scale = window.scale_factor();
            let viewport = window.viewport_size();
            let (width, height) = (
                viewport.width.as_f32() as usize,
                viewport.height.as_f32() as usize,
            );
            let quads = window.painted_quads();
            let (mut rows, mut opaque, mut total) = (Vec::new(), 0usize, 0usize);
            for y in (0..height).step_by(12) {
                let mut row = String::new();
                for x in (0..width).step_by(12) {
                    let (sx, sy) = ((x as f32 + 6.0) * scale, (y as f32 + 6.0) * scale);
                    let mut covered = 0.0f32;
                    for quad in &quads {
                        let bounds = quad.bounds;
                        if sx >= bounds.left().as_f32()
                            && sx < bounds.right().as_f32()
                            && sy >= bounds.top().as_f32()
                            && sy < bounds.bottom().as_f32()
                            && let Some(fill) = quad.background.as_solid()
                        {
                            covered += fill.to_rgb().a * (1.0 - covered);
                        }
                    }
                    total += 1;
                    opaque += usize::from(covered >= 0.95);
                    row.push(match covered {
                        c if c < 0.05 => ' ',
                        c if c < 0.3 => '.',
                        c if c < 0.6 => ':',
                        c if c < 0.85 => 'o',
                        c if c < 0.95 => 'O',
                        _ => '#',
                    });
                }
                rows.push(row);
            }
            (rows, opaque as f32 / total as f32)
        })
    }

    #[gpui_kit::test]
    fn acrylic_global_settings_window_stays_frosted_in_every_region(cx: &mut TestAppContext) {
        use nxtools_protocol::window_material::*;
        let (_app, _view, mut surface, root) = fixture(cx);
        for preference in [ThemePreference::Light, ThemePreference::Dark] {
            for page in [0usize, 3] {
                surface.update(|window, cx| {
                    let handle = window.window_handle();
                    crate::ui::material::runtime::set_test_window_handle(handle, 6351, cx);
                    let mut state = crate::ui::material::UiMaterialCapabilities::default();
                    state.connected();
                    state.apply(SystemMaterialCapabilitiesChanged {
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
                    });
                    cx.set_global(state);
                    crate::theme::set_appearance_preferences(
                        preference,
                        MaterialStyle::Acrylic,
                        cx,
                    );
                    let command = crate::ui::material::command(6351, MaterialSurface::Backdrop, cx)
                        .expect("facts");
                    crate::ui::material::paint::publish(
                        command,
                        AppliedWindowMaterial {
                            session_id: command.session_id,
                            facts_revision: command.facts_revision,
                            window_handle: command.window_handle,
                            generation: command.generation,
                            style: command.style,
                            surface: command.surface,
                            dark: command.dark,
                            effective: EffectiveWindowMaterial::Acrylic,
                            fallback_reason: MaterialFallbackReason::None,
                            native_error_code: 0,
                        },
                        cx,
                    );
                });
                select_page(&mut surface, page);
                surface.refresh().expect("acrylic settings");
                let (rows, opaque) = coverage(&mut surface);
                let at = |row: usize, column: usize| rows[row].as_bytes()[column] as char;
                let last = rows.len() - 1;
                assert!(
                    opaque < 0.15,
                    "{preference:?} page {page}: {opaque:.3} of the window is opaque"
                );
                // Content plane, sidebar and footer keep desktop-through coverage below 0.95.
                assert_ne!(at(20, 30), '#', "{preference:?} page {page}: content");
                assert_ne!(at(20, 5), '#', "{preference:?} page {page}: sidebar");
                assert_ne!(at(last, 20), '#', "{preference:?} page {page}: footer");
            }
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
