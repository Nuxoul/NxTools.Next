//! Audit #6 continuation: production Root/NewDownloadView base-font regressions.
use super::*;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::{AppContext, TestAppContext, VisualTestContext};

struct RemFixture {
    application: Entity<DownloadApplication>,
    view: Entity<NewDownloadView>,
    surface: VisualTestContext,
    _directory: tempfile::TempDir,
    _endpoint: nxtools_ipc::PlatformServerEndpoint,
}

fn fixture(cx: &mut TestAppContext, confirmed: bool) -> RemFixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated rem fixture");
    let paths = directory.path().to_path_buf();
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|cx| {
            DownloadApplication::new(
                DownloadEngine::open_in_memory().expect("memory engine, no transfer"),
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config: AppConfig::default(),
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: paths.join("download.sqlite3"),
                    ffmpeg_install_directory: paths.join("tools"),
                },
                cx,
            )
        })
    });
    let app = application.clone();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: app,
                    default_directory: paths,
                    default_connection_limit: 4,
                    default_adaptive_segment_policy: Default::default(),
                    categories: DownloadSettings::default().categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        if confirmed {
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.set_value("https://synthetic.invalid/rem.bin", window, cx)
                });
                view.file_name.update(cx, |input, cx| {
                    input.set_value("合成文件名.bin", window, cx)
                });
                view.confirmed_request = Some(view.request(cx).expect("synthetic request"));
                view.probe = Some(DownloadProbe {
                    resolution: None,
                    server_observation: None,
                    final_url: "https://synthetic.invalid/rem.bin".into(),
                    file_name: Some("合成文件名.bin".into()),
                    total_bytes: Some(1024),
                    supports_ranges: true,
                    content_type: None,
                    validator: Default::default(),
                    media: None,
                });
            });
        }
        *slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, surface);
    surface.simulate_resize(new_download_window_size(confirmed));
    surface.refresh().expect("actual production Root and view");
    RemFixture {
        application,
        view: retained.borrow().clone().expect("view"),
        surface,
        _directory: directory,
        _endpoint: endpoint,
    }
}

#[gpui_kit::test]
fn audit06_new_compact_and_confirmed_roles_follow_actual_root(cx: &mut TestAppContext) {
    for confirmed in [false, true] {
        let mut f = fixture(cx, confirmed);
        let original = f.surface.read(|cx| ComponentTheme::global(cx).clone());
        let default_rem = f.surface.update(|window, _| window.rem_size());
        let mut measured = Vec::new();
        for base in [16.0, 14.0, 18.0] {
            f.surface.update(|window, cx| {
                ComponentTheme::update(cx, |theme| theme.font_size = px(base));
                window.refresh();
            });
            f.surface
                .refresh()
                .expect("actual Root base font variation");
            let root = f.surface.debug_bounds("download-new-root").expect("root");
            let footer = f
                .surface
                .debug_bounds("download-new-footer")
                .expect("fixed footer");
            let input = f
                .surface
                .debug_bounds("download-new-url-field-input")
                .expect("original Input");
            let paste = f
                .surface
                .debug_bounds("download-new-url-paste")
                .expect("original paste Button");
            let cancel = f
                .surface
                .debug_bounds("download-new-cancel-action")
                .expect("original Cancel");
            let content = if confirmed {
                Some(
                    f.surface
                        .debug_bounds("download-new-probe-capability")
                        .expect("actual inspector text"),
                )
            } else {
                assert!(
                    f.surface
                        .debug_bounds("download-new-probe-guidance")
                        .is_none()
                );
                None
            };
            measured.push((base, root, footer, input, paste, cancel, content));
        }
        f.surface
            .update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
        assert_eq!(default_rem, px(16.0), "actual product design base");
        for (base, root, footer, input, paste, cancel, content) in measured {
            let ratio = base / 16.0;
            if let Some(content) = content {
                assert!(
                    (content.size.height - px(18.0 * ratio)).abs() <= px(0.5),
                    "ordinary content line follows rem confirmed={confirmed} base={base}: {content:?}"
                );
                assert!(content.bottom() <= footer.top());
            }
            assert_eq!(
                root.size,
                new_download_window_size(confirmed),
                "native budget is not a scale role"
            );
            assert_eq!(footer.size.height, px(44.0));
            assert_eq!(input.size.height, px(32.0 * ratio));
            if confirmed {
                assert_eq!(paste.size.height, input.size.height);
                assert_eq!(paste.top(), input.top());
            } else {
                assert_eq!(paste.size.height, px(24.0 * ratio));
                assert!(paste.top() >= input.top() && paste.bottom() <= input.bottom());
            }
            assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
        }
        assert!(
            f.application
                .read_with(&f.surface, |app, _| app.tasks().is_empty())
        );
        assert!(
            !f.view
                .read_with(&f.surface, |view, _| view.probing || view.submitting)
        );
    }
}

#[gpui_kit::test]
fn audit06_new_long_content_scrolls_without_moving_fixed_actions(cx: &mut TestAppContext) {
    for confirmed in [false, true] {
        for base in [16.0, 14.0, 18.0] {
            let mut f = fixture(cx, confirmed);
            let original = f.surface.read(|cx| ComponentTheme::global(cx).clone());
            let long_name = "合成中文文件名称与完整后缀".repeat(24) + ".bin";
            let long_path = "D:\\合成目录\\".to_owned() + &"长中文路径层级\\".repeat(24);
            let long_error = "请检查下载地址与保存路径：合成中文错误详情，所有内容必须可读。\n"
                .repeat(40)
                + "完整错误末尾";
            f.surface.update(|window, cx| {
                ComponentTheme::update(cx, |theme| theme.font_size = px(base));
                f.view.update(cx, |view, cx| {
                    view.file_name.update(cx, |input, cx| {
                        input.set_value(long_name.clone(), window, cx)
                    });
                    view.directory.update(cx, |input, cx| {
                        input.set_value(long_path.clone(), window, cx)
                    });
                    view.error = Some(long_error.clone());
                    cx.notify();
                });
            });
            f.surface.refresh().expect("long production content");
            let root = f
                .surface
                .debug_bounds("download-new-root")
                .expect("fixed root");
            let title = f
                .surface
                .debug_bounds("download-new-titlebar")
                .expect("fixed title");
            let footer = f
                .surface
                .debug_bounds("download-new-footer")
                .expect("fixed footer");
            let viewport = gpui_kit::Bounds::from_corners(
                gpui_kit::point(root.left(), title.bottom()),
                gpui_kit::point(root.right(), footer.top()),
            );
            let selector = if confirmed {
                "download-inline-error"
            } else {
                "download-new-probe-guidance"
            };
            let before = f.surface.debug_bounds(selector).expect("full error layout");
            for _ in 0..60 {
                f.surface.simulate_event(gpui_kit::ScrollWheelEvent {
                    position: viewport.center(),
                    delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -8.0)),
                    ..Default::default()
                });
                f.surface.refresh().expect("actual body wheel scroll");
            }
            let after = f
                .surface
                .debug_bounds(selector)
                .expect("current error layout");
            let cancel = f
                .surface
                .debug_bounds("download-new-cancel-action")
                .expect("Cancel hit");
            f.view.read_with(&f.surface, |view, cx| {
                assert_eq!(view.file_name.read(cx).value().as_str(), long_name);
                assert_eq!(view.directory.read(cx).value().as_str(), long_path);
                assert_eq!(view.error.as_deref(), Some(long_error.as_str()));
                assert!(!view.probing && !view.submitting);
            });
            f.surface
                .update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
            assert!(
                before.size.height > viewport.size.height,
                "complete error must lay out, not single-line truncate confirmed={confirmed} base={base}: {before:?}"
            );
            assert!(
                after.top() < before.top(),
                "real wheel scrolls complete error confirmed={confirmed} base={base}"
            );
            assert!(
                after.bottom() <= viewport.bottom() + px(0.5) && after.bottom() > viewport.top(),
                "complete error end reachable confirmed={confirmed} base={base}: {after:?}, {viewport:?}"
            );
            assert_eq!(
                root.size,
                size(px(560.0), px(if confirmed { 246.0 } else { 136.0 }))
            );
            assert_eq!(title.size.height, px(34.0));
            assert_eq!(footer.size.height, px(44.0));
            assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
            assert_eq!(f.surface.debug_bounds("download-new-footer"), Some(footer));
            assert!(
                f.application
                    .read_with(&f.surface, |app, _| app.tasks().is_empty())
            );
        }
    }
}

thread_local! {
    static TYPE_PAINT: RefCell<std::collections::HashMap<&'static str, (gpui_kit::Pixels, gpui_kit::Pixels)>> = RefCell::new(std::collections::HashMap::new());
}

pub(super) fn type_probe(role: &'static str) -> AnyElement {
    gpui_kit::canvas(
        |_, _, _| (),
        move |_, (), window, _| {
            let style = window.text_style();
            let font = style.font_size.to_pixels(window.rem_size());
            let line = style
                .line_height
                .to_pixels(style.font_size, window.rem_size());
            TYPE_PAINT.with(|paint| paint.borrow_mut().insert(role, (font, line)));
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

#[gpui_kit::test]
fn audit06_new_painted_type_and_basic_control_rails_follow_root(cx: &mut TestAppContext) {
    for confirmed in [false, true] {
        let mut f = fixture(cx, confirmed);
        let original = f.surface.read(|cx| ComponentTheme::global(cx).clone());
        let mut default_label_line = None;
        for base in [16.0, 14.0, 18.0] {
            TYPE_PAINT.with(|paint| paint.borrow_mut().clear());
            f.surface.update(|window, cx| {
                ComponentTheme::update(cx, |theme| theme.font_size = px(base));
                f.view.update(cx, |_, cx| cx.notify());
                window.refresh();
                window.draw(cx).clear(cx);
            });
            let types = TYPE_PAINT.with(|paint| paint.borrow().clone());
            let ratio = base / 16.0;
            for role in if confirmed {
                vec!["status", "size", "capability"]
            } else {
                vec![]
            } {
                assert_eq!(
                    types.get(role),
                    Some(&(px(12.0 * ratio), px(18.0 * ratio))),
                    "paint role={role} base={base}"
                );
            }
            if confirmed {
                let (label_font, label_line) = types
                    .get("download-new-url-field")
                    .copied()
                    .expect("actual official label paint");
                assert_eq!(label_font, px(14.0 * ratio), "official text_sm label font");
                let baseline_line = *default_label_line.get_or_insert(label_line);
                assert!(
                    (label_line - baseline_line * ratio).abs() < px(0.02),
                    "original inherited label line follows font, not a new forced type role"
                );
            }
            let url = f
                .surface
                .debug_bounds("download-new-url-control")
                .expect("URL rail");
            let label = f.surface.debug_bounds("download-new-url-field-label");
            let paste = f
                .surface
                .debug_bounds("download-new-url-paste")
                .expect("paste");
            let input = f
                .surface
                .debug_bounds("download-new-url-field-input")
                .expect("input");
            if confirmed {
                assert_eq!(label.expect("confirmed label").left(), px(16.0 * ratio));
                assert_eq!(
                    url.left(),
                    px(88.0 * ratio) + px(4.0),
                    "72 rem label plus released fixed4 Field gap"
                );
            } else {
                assert!(label.is_none(), "compact input has no separate label rail");
                assert_eq!(url.left(), px(16.0 * ratio));
            }
            assert_eq!(url.size.height, px(32.0 * ratio));
            if confirmed {
                assert!((paste.left() - input.right() - px(8.0 * ratio)).abs() < px(0.5));
            } else {
                assert!(paste.left() >= input.left() && paste.right() <= input.right());
            }
            if base == 16.0 {
                assert_eq!(input.top(), px(if confirmed { 42.0 } else { 38.0 }));
                assert_eq!(paste.right(), px(if confirmed { 544.0 } else { 539.0 }));
            }
            if confirmed {
                let directory = f
                    .surface
                    .debug_bounds("download-new-directory-control")
                    .expect("directory rail");
                let directory_input = f
                    .surface
                    .debug_bounds("download-new-directory-input")
                    .expect("directory input");
                let picker = f
                    .surface
                    .debug_bounds("download-new-directory-picker")
                    .expect("picker");
                let inspector = f
                    .surface
                    .debug_bounds("download-new-probe-inspector")
                    .expect("inspector");
                assert_eq!(directory.left(), url.left());
                assert_eq!(directory.size.height, input.size.height);
                assert_eq!(picker.top(), directory_input.top());
                assert_eq!(picker.size.height, input.size.height);
                assert_eq!(inspector.size, size(px(120.0 * ratio), px(112.0 * ratio)));
                assert!((inspector.left() - directory.right() - px(8.0 * ratio)).abs() < px(0.5));
                if base == 16.0 {
                    assert_eq!(directory.top(), px(82.0));
                    assert_eq!(inspector.bottom(), px(194.0));
                }
            }
        }
        f.surface
            .update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
    }
}

// Traverse the same public focus sequence used by released Button tests.
fn tab_to(surface: &mut VisualTestContext, id: &'static str) {
    for _ in 0..40 {
        surface.update(|window, cx| window.focus_next(cx));
        surface.update(|window, cx| window.draw(cx).clear(cx));
        if surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.path().last() == Some(&gpui_kit::ElementId::from(id))
                        && node.focused() == Some(true)
                })
        }) {
            return;
        }
    }
    panic!("original Button {id} not reachable by Tab sequence");
}

#[gpui_kit::test]
fn audit06_new_relative_layout_keeps_tab_and_cancel_pointer_owner(cx: &mut TestAppContext) {
    for confirmed in [false, true] {
        for base in [16.0, 14.0, 18.0] {
            let mut f = fixture(cx, confirmed);
            let original = f.surface.read(|cx| ComponentTheme::global(cx).clone());
            f.surface.update(|window, cx| {
                ComponentTheme::update(cx, |theme| theme.font_size = px(base));
                window.activate_window();
                window.blur(cx);
            });
            f.surface.refresh().expect("fixed production viewport");
            tab_to(&mut f.surface, "download-paste-url");
            if confirmed {
                tab_to(&mut f.surface, "download-pick-directory");
            }
            let footer = f
                .surface
                .debug_bounds("download-new-footer")
                .expect("fixed footer");
            if confirmed {
                for (id, selector) in [
                    (
                        "download-new-schedule-trigger",
                        "download-new-footer-secondary-schedule-action",
                    ),
                    (
                        "download-add-to-queue-button",
                        "download-new-footer-secondary-queue-action",
                    ),
                    (
                        "download-start-now-button",
                        "download-new-footer-primary-action",
                    ),
                ] {
                    tab_to(&mut f.surface, id);
                    let hit = f
                        .surface
                        .debug_bounds(selector)
                        .expect("actual footer Button");
                    assert!(
                        hit.top() >= footer.top()
                            && hit.bottom() <= footer.bottom()
                            && hit.left() >= footer.left()
                            && hit.right() <= footer.right(),
                        "original action remains inside fixed footer id={id} base={base}"
                    );
                }
            }
            tab_to(&mut f.surface, "download-new-cancel");
            let cancel = f
                .surface
                .debug_bounds("download-new-cancel-action")
                .expect("Cancel hit");
            let handle = f.surface.update(|window, _| window.window_handle());
            f.surface
                .simulate_click(cancel.center(), gpui_kit::Modifiers::default());
            f.surface.run_until_parked();
            let closed = f.surface.read(|cx| !cx.windows().contains(&handle));
            cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
            assert!(
                closed,
                "original Cancel pointer callback closes exact window confirmed={confirmed} base={base}"
            );
            assert!(cx.read(|cx| f.application.read(cx).tasks().is_empty()));
        }
    }
}

#[gpui_kit::test]
fn audit06_new_long_inputs_keep_original_entity_and_keyboard_tail_access(cx: &mut TestAppContext) {
    for base in [16.0, 14.0, 18.0] {
        let mut f = fixture(cx, true);
        let original = f.surface.read(|cx| ComponentTheme::global(cx).clone());
        let values = [
            "D:\\合成目录\\".to_owned() + &"可访问的中文路径层级\\".repeat(20),
            "可访问的完整中文文件名称".repeat(24) + ".bin",
        ];
        let inputs = f.view.read_with(&f.surface, |view, _| {
            [view.directory.clone(), view.file_name.clone()]
        });
        f.surface.update(|window, cx| {
            ComponentTheme::update(cx, |theme| theme.font_size = px(base));
            for (input, value) in inputs.iter().zip(&values) {
                input.update(cx, |state, cx| state.set_value(value.clone(), window, cx));
            }
            f.view.update(cx, |_, cx| cx.notify());
        });
        f.surface.refresh().expect("long original Inputs");
        for (index, selector) in [
            "download-new-directory-input",
            "download-new-file-name-control",
        ]
        .iter()
        .enumerate()
        {
            let hit = f
                .surface
                .debug_bounds(selector)
                .expect("original Input hit");
            let footer = f
                .surface
                .debug_bounds("download-new-footer")
                .expect("footer");
            assert!(
                hit.top() >= px(34.0) && hit.bottom() <= footer.top(),
                "long input remains reachable base={base}"
            );
            f.surface
                .simulate_click(hit.center(), gpui_kit::Modifiers::default());
            assert!(
                f.surface
                    .update(|window, cx| inputs[index].focus_handle(cx).is_focused(window)),
                "pointer focuses original Input"
            );
            f.surface.simulate_keystrokes("end");
            f.surface.simulate_input("末");
            assert_eq!(
                inputs[index].read_with(&f.surface, |state, _| state.value()),
                values[index].clone() + "末",
                "real keyboard reaches full long-text tail"
            );
            f.surface.simulate_keystrokes("backspace");
            assert_eq!(
                inputs[index].read_with(&f.surface, |state, _| state.value()),
                values[index]
            );
            let retained = f.view.read_with(&f.surface, |view, _| {
                if index == 0 {
                    view.directory.clone()
                } else {
                    view.file_name.clone()
                }
            });
            assert_eq!(retained.entity_id(), inputs[index].entity_id());
        }
        f.surface
            .update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
        assert!(
            f.application
                .read_with(&f.surface, |app, _| app.tasks().is_empty())
        );
        assert!(
            !f.view
                .read_with(&f.surface, |view, _| view.probing || view.submitting)
        );
    }
}
