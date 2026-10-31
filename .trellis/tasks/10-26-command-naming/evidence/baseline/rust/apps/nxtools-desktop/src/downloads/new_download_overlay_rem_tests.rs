//! Queue05: actual production Root overlay/media geometry and input ownership.
use super::*;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};

struct Fixture {
    application: Entity<DownloadApplication>,
    view: Entity<NewDownloadView>,
    surface: VisualTestContext,
    _directory: tempfile::TempDir,
    _endpoint: nxtools_ipc::PlatformServerEndpoint,
}

fn fixture(
    cx: &mut TestAppContext,
    base: f32,
    media: Option<nxtools_download::DownloadMediaProbe>,
) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.update(|cx| ComponentTheme::update(cx, |theme| theme.font_size = px(base)));
    let directory = tempfile::tempdir().expect("isolated overlay fixture");
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
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let probe = DownloadProbe {
        resolution: None,
        final_url: "https://synthetic.invalid/rem.bin".into(),
        file_name: Some("合成文件.bin".into()),
        total_bytes: Some(1024),
        supports_ranges: true,
        content_type: None,
        validator: Default::default(),
        media,
    };
    let dimensions = new_download_window_size_for_probe(&probe);
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: app,
                    default_directory: paths,
                    default_connection_limit: 4,
                    default_adaptive_segment_policy: Default::default(),
                    categories: (0..32)
                        .map(|index| DownloadCategorySettings {
                            id: format!("synthetic-{index}"),
                            name: format!("合成分类{index:02}中文长标题"),
                            save_directory: format!(
                                "D:\\合成目录\\{index:02}\\{}",
                                "完整中文路径\\".repeat(18)
                            ),
                            enabled: true,
                            extensions: Vec::new(),
                            url_patterns: Vec::new(),
                        })
                        .collect(),
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        view.update(cx, |view, cx| {
            view.url.update(cx, |input, cx| {
                input.set_value("https://synthetic.invalid/rem.bin", window, cx)
            });
            view.file_name
                .update(cx, |input, cx| input.set_value("合成文件.bin", window, cx));
            view.confirmed_request = Some(view.request(cx).expect("synthetic request"));
            view.selected_hls_variant = Some(0);
            view.selected_dash_video_track = Some(0);
            view.selected_dash_audio_track = Some(0);
            view.probe = Some(probe);
        });
        *retained.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let mut surface =
        VisualTestContext::from_window(surface.update(|window, _| window.window_handle()), surface);
    surface.simulate_resize(dimensions);
    draw(&mut surface);
    Fixture {
        application,
        view: slot.borrow().clone().expect("production view"),
        surface,
        _directory: directory,
        _endpoint: endpoint,
    }
}

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn click(surface: &mut VisualTestContext, selector: &'static str) {
    let hit = surface.debug_bounds(selector).expect(selector);
    surface.simulate_click(hit.center(), Modifiers::default());
    draw(surface);
}

fn key(surface: &mut VisualTestContext, key: &str) {
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("test key"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("test release"),
    });
    draw(surface);
}

fn no_outer(f: &Fixture) {
    f.view.read_with(&f.surface, |view, _| {
        assert!(!view.probing && !view.submitting);
        assert!(
            view.error.is_none(),
            "child must not invoke outer validation: {:?}",
            view.error
        );
    });
    assert!(
        f.application
            .read_with(&f.surface, |app, _| app.tasks().is_empty())
    );
}

#[gpui_kit::test]
fn audit06_overlay_category_rows_and_search_follow_root(cx: &mut TestAppContext) {
    let original = capture_theme(cx);
    for base in [16.0, 14.0, 18.0] {
        let mut f = fixture(cx, base, None);
        click(&mut f.surface, "download-new-category-trigger");
        let ratio = base / 16.0;
        let row = f
            .surface
            .debug_bounds("download-new-category-option-0")
            .expect("actual ListItem");
        let title = f
            .surface
            .debug_bounds("download-new-category-option-title-0")
            .expect("title");
        let detail = f
            .surface
            .debug_bounds("download-new-category-option-directory-0")
            .expect("detail");
        let search = f
            .surface
            .debug_bounds("download-new-category-search")
            .expect("search");
        let list = f
            .surface
            .debug_bounds("download-new-category-list")
            .expect("list");
        let shell = f
            .surface
            .debug_bounds("download-new-category-popover-content")
            .expect("bounded category content");
        assert_eq!(shell.size.width + px(24.0 * ratio), px(320.0));
        assert_eq!(shell.size.height + px(24.0 * ratio), px(240.0));
        assert!(
            (row.size.height - px(34.0 * ratio)).abs() < px(0.5),
            "ordinary category row base={base}: {row:?}"
        );
        assert!((title.size.height - px(18.0 * ratio)).abs() < px(0.5));
        assert!((detail.size.height - px(16.0 * ratio)).abs() < px(0.5));
        assert_eq!(search.size.height, px(32.0 * ratio));
        assert!(title.left() >= row.left() && title.right() <= row.right());
        assert!(title.bottom() <= detail.top() && detail.bottom() <= row.bottom());
        assert!(search.bottom() <= list.top());
        assert_eq!(
            f.view
                .read_with(&f.surface, |view, _| view.category_scroll.max_offset().x),
            px(0.0)
        );
        for _ in 0..24 {
            f.surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: list.center(),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -8.0)),
                ..Default::default()
            });
            draw(&mut f.surface);
        }
        let last = f
            .surface
            .debug_bounds("download-new-category-option-31")
            .expect("last category");
        assert!(
            last.bottom() <= list.bottom() + px(0.5) && last.top() >= list.top(),
            "actual wheel reaches final category base={base}"
        );
        f.surface
            .simulate_mouse_move(last.center(), None, Modifiers::default());
        f.surface.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(600));
        draw(&mut f.surface);
        assert!(
            f.surface
                .debug_bounds("download-new-category-option-tooltip-31")
                .is_some(),
            "original full-path Tooltip remains"
        );
        let full_label = f.view.read_with(&f.surface, |view, _| {
            format!(
                "{} · {}",
                view.categories[31].name, view.categories[31].save_directory
            )
        });
        assert!(
            f.surface
                .update(|window, _| gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .any(|node| node.label() == Some(full_label.as_str()))),
            "original category semantics retain full name and path"
        );
        f.surface
            .simulate_click(search.center(), Modifiers::default());
        f.surface.simulate_keystrokes("ctrl-a");
        f.surface.simulate_input("合成分类31");
        draw(&mut f.surface);
        assert_eq!(
            f.view
                .read_with(&f.surface, |view, cx| view.category_search.read(cx).value()),
            "合成分类31"
        );
        key(&mut f.surface, "enter");
        assert_eq!(
            f.view
                .read_with(&f.surface, |view, _| view.selected_category_id.clone()),
            Some("synthetic-31".into())
        );
        assert!(
            !f.view
                .read_with(&f.surface, |view, _| view.category_popover_open)
        );
        no_outer(&f);
        key(&mut f.surface, "escape");
    }
    cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
}

fn hls(count: usize) -> nxtools_download::DownloadMediaProbe {
    nxtools_download::DownloadMediaProbe::Hls(nxtools_download::HlsProbe {
        is_master: true,
        variants: (0..count)
            .map(|index| nxtools_download::HlsVariantProbe {
                playlist_url: format!("https://synthetic.invalid/quality-{index}.m3u8"),
                bandwidth: 1_000_000,
                average_bandwidth: None,
                resolution: Some((1920, 1080)),
                frame_rate_milli: Some(30_000),
                codecs: "avc1.640028,mp4a.40.2".into(),
                audio_group: None,
                subtitles_group: None,
            })
            .collect(),
    })
}

fn dash(count: usize) -> nxtools_download::DownloadMediaProbe {
    let tracks = |audio: bool| {
        (0..count)
            .map(|index| nxtools_download::DashTrackProbe {
                representation_id: format!("synthetic-{index}"),
                bandwidth: 1_000_000,
                resolution: (!audio).then_some((1920, 1080)),
                codecs: if audio { "mp4a.40.2" } else { "avc1.640028" }.into(),
                mime_type: if audio { "audio/mp4" } else { "video/mp4" }.into(),
                language: audio.then_some(format!("中文音轨{index:02}")),
                segment_count: 2,
                has_initialization: true,
            })
            .collect()
    };
    nxtools_download::DownloadMediaProbe::Dash(nxtools_download::DashProbe {
        video_tracks: tracks(false),
        audio_tracks: tracks(true),
    })
}

#[gpui_kit::test]
fn audit06_overlay_schedule_calendar_and_time_follow_root(cx: &mut TestAppContext) {
    let original = capture_theme(cx);
    for base in [16.0, 14.0, 18.0] {
        let mut f = fixture(cx, base, None);
        click(&mut f.surface, "download-new-schedule-control");
        let shell = f
            .surface
            .debug_bounds("download-new-schedule-popover-shell")
            .expect("schedule shell");
        let day = f
            .surface
            .debug_bounds("download-new-schedule-next-day")
            .expect("actual day Button");
        let time = f
            .surface
            .debug_bounds("download-new-schedule-time-row")
            .expect("time rail");
        let actions = f
            .surface
            .debug_bounds("download-new-schedule-action-row")
            .expect("actions");
        let ratio = base / 16.0;
        assert_eq!(shell.size.width + px(8.0 * ratio), px(304.0));
        assert_eq!(shell.size.height + px(8.0 * ratio), px(200.0));
        for selector in [
            "download-new-schedule-hour-rail",
            "download-new-schedule-minute-rail",
        ] {
            let rail = f
                .surface
                .debug_bounds(selector)
                .expect("actual Select rail");
            assert!(
                (rail.size.width - px(66.0 * ratio)).abs() <= px(0.5),
                "official Select rail pixel alignment"
            );
            assert_eq!(rail.size.height, px(24.0 * ratio));
        }
        if base == 16.0 {
            assert_eq!(day.size, size(px(40.0), px(19.0)));
        }
        assert!(
            (day.size.height - px(19.0 * ratio)).abs() < px(0.5),
            "ordinary calendar day base={base}: {day:?}"
        );
        assert!(day.left() >= shell.left() && day.right() <= shell.right());
        assert!(
            time.bottom() <= actions.top() && actions.bottom() <= shell.bottom(),
            "all footer actions inside fixed Popover base={base}: time={time:?}, actions={actions:?}, shell={shell:?}"
        );
        no_outer(&f);
        key(&mut f.surface, "escape");
    }
    cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
}

#[gpui_kit::test]
fn audit06_overlay_media_rows_and_inspector_follow_root(cx: &mut TestAppContext) {
    let original = capture_theme(cx);
    for is_dash in [false, true] {
        for base in [16.0, 14.0, 18.0] {
            let mut f = fixture(cx, base, Some(if is_dash { dash(8) } else { hls(8) }));
            let row = f
                .surface
                .debug_bounds(if is_dash {
                    "download-dash-video-track-0"
                } else {
                    "download-hls-quality-0"
                })
                .expect("actual Radio");
            let inspector = f
                .surface
                .debug_bounds("download-new-probe-inspector")
                .expect("media inspector");
            let root = f
                .surface
                .debug_bounds("download-new-root")
                .expect("native budget");
            let footer = f
                .surface
                .debug_bounds("download-new-footer")
                .expect("fixed footer");
            let ratio = base / 16.0;
            assert!(
                (row.size.height - px(40.0 * ratio)).abs() < px(0.5),
                "ordinary media row dash={is_dash} base={base}: {row:?}"
            );
            assert_eq!(inspector.size, size(px(120.0 * ratio), px(112.0 * ratio)));
            assert_eq!(
                root.size,
                size(px(560.0), px(if is_dash { 433.0 } else { 418.0 }))
            );
            assert_eq!(footer.size.height, px(44.0));
            no_outer(&f);
        }
    }
    cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
}

fn tab_until(
    surface: &mut VisualTestContext,
    predicate: impl Fn(&gpui_kit::base::test_support::ElementSnapshot) -> bool,
) {
    for _ in 0..80 {
        key(surface, "tab");
        if surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.focused() == Some(true) && predicate(node))
        }) {
            return;
        }
    }
    let observed = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|node| {
                node.focused() == Some(true) || node.role() == Some(gpui_kit::Role::RadioButton)
            })
            .map(|node| {
                format!(
                    "{:?} focused={:?} path={:?}",
                    node.label(),
                    node.focused(),
                    node.path()
                )
            })
            .collect::<Vec<_>>()
    });
    panic!("actual Tab failed to reach requested production control: {observed:?}");
}

fn tab_button(surface: &mut VisualTestContext, id: &'static str) {
    tab_until(surface, |node| {
        node.path().last() == Some(&gpui_kit::ElementId::from(id))
    });
}

fn wheel_end(surface: &mut VisualTestContext, viewport: gpui_kit::Bounds<gpui_kit::Pixels>) {
    for _ in 0..30 {
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: viewport.center(),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -8.0)),
            ..Default::default()
        });
        draw(surface);
    }
}

#[gpui_kit::test]
fn audit06_overlay_schedule_last_date_select_confirm_clear_and_cancel_keep_owners(
    cx: &mut TestAppContext,
) {
    let original = capture_theme(cx);
    for base in [16.0, 14.0, 18.0] {
        let mut f = fixture(cx, base, None);
        click(&mut f.surface, "download-new-schedule-control");
        let viewport = f
            .surface
            .debug_bounds("download-new-schedule-calendar-scroll")
            .expect("original calendar viewport");
        assert!(viewport.size.height <= px(124.0));
        let last_date = f.view.read_with(&f.surface, |view, _| {
            delayed_start_calendar_days(view.scheduled_month)[41]
        });
        let before = f
            .surface
            .debug_bounds("download-new-schedule-last-day")
            .expect("last date before wheel");
        wheel_end(&mut f.surface, viewport);
        let after = f
            .surface
            .debug_bounds("download-new-schedule-last-day")
            .expect("last date after wheel");
        assert!(
            after.top() < before.top()
                && after.top() >= viewport.top()
                && after.bottom() <= viewport.bottom() + px(0.5),
            "actual last date visible base={base}: {after:?}, {viewport:?}"
        );
        click(&mut f.surface, "download-new-schedule-last-day");
        assert_eq!(
            f.view.read_with(&f.surface, |view, _| view.scheduled_date),
            last_date
        );
        let hour = f
            .view
            .read_with(&f.surface, |view, _| view.scheduled_hour.clone());
        let previous_hour = hour.read_with(&f.surface, |state, cx| state.selected_index(cx));
        click(&mut f.surface, "download-new-schedule-hour-rail");
        key(&mut f.surface, "down");
        key(&mut f.surface, "down");
        key(&mut f.surface, "enter");
        assert_ne!(
            hour.read_with(&f.surface, |state, cx| state.selected_index(cx)),
            previous_hour,
            "real Select draft confirmation"
        );
        assert!(
            f.view
                .read_with(&f.surface, |view, _| view.schedule_popover_open
                    && view.committed_schedule.is_none())
        );
        no_outer(&f);
        tab_button(&mut f.surface, "download-new-schedule-confirm-button");
        key(&mut f.surface, "enter");
        let committed = f
            .view
            .read_with(&f.surface, |view, _| view.committed_schedule);
        assert_eq!(
            committed.expect("original deferred confirm").date,
            last_date
        );
        no_outer(&f);
        click(&mut f.surface, "download-new-schedule-control");
        click(&mut f.surface, "download-new-schedule-next-month-action");
        key(&mut f.surface, "escape");
        assert_eq!(
            f.view
                .read_with(&f.surface, |view, _| view.committed_schedule),
            committed,
            "cancel preserves original committed selection"
        );
        click(&mut f.surface, "download-new-schedule-control");
        tab_button(&mut f.surface, "download-new-schedule-clear");
        key(&mut f.surface, "space");
        assert!(
            f.view
                .read_with(&f.surface, |view, _| view.committed_schedule.is_none()
                    && !view.schedule_popover_open)
        );
        no_outer(&f);
    }
    cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
}

#[gpui_kit::test]
fn audit06_overlay_media_long_last_item_wheel_and_release_keep_original_selection(
    cx: &mut TestAppContext,
) {
    let original = capture_theme(cx);
    for is_dash in [false, true] {
        for base in [16.0, 14.0, 18.0] {
            let mut media = if is_dash { dash(8) } else { hls(8) };
            let long = "合成中文完整轨道元数据".repeat(8);
            match &mut media {
                nxtools_download::DownloadMediaProbe::Hls(probe) => {
                    probe.variants[7].codecs = long.clone()
                }
                nxtools_download::DownloadMediaProbe::Dash(probe) => {
                    probe.audio_tracks[7].language = Some(long.clone());
                    probe.audio_tracks[7].codecs = long.clone();
                }
            }
            let mut f = fixture(cx, base, Some(media));
            let viewport = f
                .surface
                .debug_bounds(if is_dash {
                    "download-dash-audio-scroll"
                } else {
                    "download-hls-quality-scroll"
                })
                .expect("original media viewport");
            assert!(viewport.size.height <= px(120.0));
            let selector = if is_dash {
                "download-dash-audio-track-7"
            } else {
                "download-hls-quality-7"
            };
            let detail_selector = if is_dash {
                "download-dash-audio-track-7-detail"
            } else {
                "download-hls-quality-7-detail"
            };
            let before = f
                .surface
                .debug_bounds(selector)
                .expect("last Radio before wheel");
            wheel_end(&mut f.surface, viewport);
            let after = f
                .surface
                .debug_bounds(selector)
                .expect("last Radio after wheel");
            let detail = f
                .surface
                .debug_bounds(detail_selector)
                .expect("full long detail");
            assert!(
                after.top() < before.top(),
                "actual media wheel moves final row dash={is_dash} base={base}"
            );
            assert!(
                detail.size.height > px(15.0 * base / 16.0),
                "long metadata wraps, not hides base={base}"
            );
            assert!(
                detail.bottom() <= viewport.bottom() + px(0.5) && detail.bottom() > viewport.top(),
                "full last metadata end reachable dash={is_dash} base={base}: row={after:?} detail={detail:?} viewport={viewport:?}"
            );
            assert!(
                detail.left() >= viewport.left() && detail.right() <= viewport.right() + px(0.5)
            );
            let hit = gpui_kit::point(viewport.center().x, detail.bottom() - px(3.0));
            f.surface.simulate_click(hit, Modifiers::default());
            draw(&mut f.surface);
            assert_eq!(
                f.view.read_with(&f.surface, |view, _| if is_dash {
                    view.selected_dash_audio_track
                } else {
                    view.selected_hls_variant
                }),
                Some(7)
            );
            if is_dash {
                let group = gpui_kit::ElementId::from("音频");
                click(&mut f.surface, "download-new-file-name-control");
                tab_until(&mut f.surface, |node| {
                    node.role() == Some(gpui_kit::Role::RadioButton)
                        && node.path().contains(&group)
                        && node.path().last() == Some(&gpui_kit::ElementId::from(7usize))
                });
                key(&mut f.surface, "space");
                assert!(
                    f.view.read_with(&f.surface, |view, _| view
                        .selected_dash_audio_track
                        .is_none()),
                    "original nullable audio callback clears once on release"
                );
            }
            no_outer(&f);
        }
    }
    cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
}

fn capture_theme(cx: &mut TestAppContext) -> ComponentTheme {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.read(|cx| ComponentTheme::global(cx).clone())
}
