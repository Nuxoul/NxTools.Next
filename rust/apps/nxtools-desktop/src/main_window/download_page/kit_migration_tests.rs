use super::*;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::{AppContext, Focusable as _, Modifiers, TestAppContext, VisualTestContext, size};

struct GridDelegate {
    external: Option<Rc<RefCell<HashSet<usize>>>>,
}
impl TableDelegate for GridDelegate {
    fn columns_count(&self, _: &App) -> usize {
        1
    }
    fn rows_count(&self, _: &App) -> usize {
        3
    }
    fn column(&self, _: usize, _: &App) -> Column {
        Column::new("synthetic", "Synthetic")
            .width(px(200.0))
            .sortable()
    }
    fn render_header(
        &mut self,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        div()
            .id("header")
            .debug_selector(|| "candidate-header".into())
    }
    fn perform_sort(
        &mut self,
        col: usize,
        _: gpui_kit::component::table::ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        if let Some(external) = &self.external {
            external.borrow_mut().insert(100 + col);
        }
    }
    fn context_menu(
        &mut self,
        row: usize,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        if let Some(external) = &self.external {
            external.borrow_mut().insert(200 + row);
        }
        menu.item(PopupMenuItem::new("Synthetic action").on_click(move |_, _, _| {}))
    }
    fn render_tr(
        &mut self,
        row: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let table_id = cx.entity_id();
        let selected = self
            .external
            .as_ref()
            .is_some_and(|set| set.borrow().contains(&(row + 10)));
        div()
            .id(("row", row))
            .debug_selector(move || format!("candidate-row-{row}"))
            .aria_selected(selected)
            .when_some(self.external.clone(), |tr, external| {
                tr.on_click(move |event, _, cx| {
                    let mut set = external.borrow_mut();
                    if !event.modifiers().control {
                        set.clear();
                    }
                    if !set.insert(row + 10) {
                        set.remove(&(row + 10));
                    }
                    cx.notify(table_id);
                })
            })
    }
    fn render_td(
        &mut self,
        row: usize,
        _: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().child(format!("row {row}"))
    }
}
struct GridFixture {
    state: Entity<TableState<GridDelegate>>,
}
impl Render for GridFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        DataTable::new(&self.state)
    }
}
#[gpui_kit::test]
fn queue03_datatable_ctrl_click_exposes_single_index_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let state = cx.new(|cx| TableState::new(GridDelegate { external: None }, window, cx));
        *retained.borrow_mut() = Some(state.clone());
        let view = cx.new(|_| GridFixture { state });
        gpui_kit::base::Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(400.0), px(260.0)));
    window.refresh().expect("released table");
    let first = window
        .debug_bounds("candidate-row-0")
        .expect("first actual row");
    let second = window
        .debug_bounds("candidate-row-1")
        .expect("second actual row");
    window.simulate_click(first.center(), Modifiers::default());
    window.simulate_click(
        second.center(),
        Modifiers {
            control: true,
            ..Default::default()
        },
    );
    window.run_until_parked();
    let state = slot.borrow_mut().take().expect("table state");
    assert_eq!(
        window.read(|cx| state.read(cx).selected_row()),
        Some(1),
        "public built-in selection is one index, not stable-ID multiselect"
    );
    window.refresh().expect("selected row semantics");
    let selected = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .filter(|snapshot| {
            snapshot.role() == Some(gpui_kit::Role::Row) && snapshot.selected() == Some(true)
        })
        .count();
    assert_eq!(selected, 1);
}

fn workspace(
    cx: &mut TestAppContext,
) -> (
    Entity<MainWindow>,
    gpui_kit::AnyWindowHandle,
    tempfile::TempDir,
    Entity<DownloadApplication>,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let temp = tempfile::tempdir().expect("isolated download workspace");
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("memory engine");
    let mut request = nxtools_download::DownloadRequest {
        source_url: "https://synthetic.invalid/item".into(),
        save_directory: temp.path().into(),
        save_directory_explicit: false,
        file_name: Some("SyntheticLongName".repeat(15)),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    let task_id = engine
        .enqueue(request.clone(), RequestOrigin::Internal)
        .expect("queue-only synthetic task");
    request.file_name = Some("Synthetic-second.bin".into());
    engine
        .enqueue(request, RequestOrigin::Internal)
        .expect("second queue-only synthetic task");
    let (platform, _) = nxtools_ipc::platform_bridge(16);
    let paths = temp.path().to_owned();
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications.clone(),
                crate::downloads::DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config: nxtools_core::AppConfig::default(),
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: paths.join("downloads.sqlite3"),
                    ffmpeg_install_directory: paths.join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let retained_downloads = downloads.clone();
    let (main, window) = cx.add_window_view(move |window, cx| {
        let mut main = MainWindow::new(
            gpui_kit::WeakEntity::new_invalid(),
            gpui_kit::WeakEntity::new_invalid(),
            downloads.downgrade(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main.shell.activate(MainModule::Downloads);
        main.selected_download_task = Some(task_id);
        main.selected_download_tasks.insert(task_id);
        main.requested_download_detail = Some(task_id);
        main.download_detail_scope = Some(crate::downloads::DownloadDetailScope::new(task_id));
        main
    });
    let handle = window.update(|window, _| window.window_handle());
    (main, handle, temp, retained_downloads)
}
#[gpui_kit::test]
fn queue03_workspace_mounts_real_compounds_at_default_and_minimum(cx: &mut TestAppContext) {
    let (main, handle, _temp, _downloads) = workspace(cx);
    let mut window = VisualTestContext::from_window(handle, cx);
    for dimensions in [(1060.0, 650.0), (920.0, 480.0)] {
        window.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        window.refresh().expect("production compound workspace");
        let snapshots = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        let group = snapshots
            .iter()
            .find(|snapshot| {
                snapshot.path().last() == Some(&gpui_kit::ElementId::from("download-search-group"))
            })
            .expect("actual InputGroup")
            .bounds();
        assert_eq!(group.size.height, px(32.0));
        assert!(group.right() <= px(dimensions.0));
        let commands: Vec<_> = snapshots
            .iter()
            .filter(|snapshot| {
                snapshot.role() == Some(gpui_kit::Role::Button)
                    && snapshot
                        .path()
                        .contains(&gpui_kit::ElementId::from("download-header-toolbar"))
                    && snapshot
                        .path()
                        .contains(&gpui_kit::ElementId::from("download-header-command-group"))
            })
            .collect();
        assert_eq!(
            commands.len(),
            3,
            "actual Toolbar/ToolbarGroup command descendants"
        );
        for command in commands {
            assert!(command.bounds().right() <= px(dimensions.0));
        }
        assert_eq!(
            snapshots
                .iter()
                .filter(|snapshot| snapshot.role() == Some(gpui_kit::Role::Tab))
                .count(),
            5,
            "all five official tabs"
        );
        let track = window
            .debug_bounds("download-task-progress-track-selected")
            .expect("actual aggregate track");
        assert_eq!(
            track.size.height,
            px(4.0),
            "official XSmall Progress geometry"
        );
        let input = main.read_with(&window, |main, _| main.download_search_input.clone());
        let identity = input.entity_id();
        let icon = snapshots
            .iter()
            .find(|snapshot| {
                snapshot.path().last() == Some(&gpui_kit::ElementId::from("download-search-icon"))
            })
            .expect("Search addon")
            .bounds();
        window.simulate_click(icon.center(), Modifiers::default());
        window.simulate_input("Synthetic");
        window.run_until_parked();
        window.refresh().expect("query clear addon");
        let clear = window
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .into_iter()
            .find(|snapshot| {
                snapshot.path().last() == Some(&gpui_kit::ElementId::from("download-search-clear"))
            })
            .expect("original InputGroupButton clear action")
            .bounds();
        window.simulate_click(clear.center(), Modifiers::default());
        window.run_until_parked();
        assert!(window.read(|cx| input.read(cx).value().is_empty()));
        assert_eq!(
            main.read_with(&window, |main, _| main.download_search_input.entity_id()),
            identity
        );
    }
}

#[gpui_kit::test]
fn queue03_public_row_override_keeps_external_set_but_overwrites_selection_semantics(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let selected = Rc::new(RefCell::new(HashSet::new()));
    let external = selected.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let state = cx.new(|cx| {
            TableState::new(
                GridDelegate {
                    external: Some(external),
                },
                window,
                cx,
            )
            .row_selectable(false)
        });
        let view = cx.new(|_| GridFixture { state });
        gpui_kit::base::Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(400.0), px(260.0)));
    window.refresh().expect("public row override");
    let first = window.debug_bounds("candidate-row-0").expect("first row");
    let second = window.debug_bounds("candidate-row-1").expect("second row");
    window.simulate_click(first.center(), Modifiers::default());
    window.simulate_click(
        second.center(),
        Modifiers {
            control: true,
            ..Default::default()
        },
    );
    window.run_until_parked();
    window.refresh().expect("external selection repaint");
    assert_eq!(
        *selected.borrow(),
        HashSet::from([10, 11]),
        "public row callback can retain original ID set"
    );
    let semantic = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .filter(|snapshot| {
            snapshot.role() == Some(gpui_kit::Role::Row) && snapshot.selected() == Some(true)
        })
        .count();
    assert_eq!(
        semantic, 0,
        "release overwrites delegate aria_selected with disabled internal selection; not an unsupported-callback claim"
    );
}

#[gpui_kit::test]
fn queue03_metadata_uses_natural_description_list_layout(cx: &mut TestAppContext) {
    let (main, handle, _temp, _downloads) = workspace(cx);
    let mut window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("production overview metadata");
    let field = window
        .debug_bounds("download-detail-field-downloaded")
        .expect("downloaded metadata");
    let label = window
        .debug_bounds("download-detail-field-downloaded-label")
        .expect("description label");
    let value = window
        .debug_bounds("download-detail-field-downloaded-value")
        .expect("description value");
    assert_eq!(
        label.size.height,
        px(22.5),
        "released text_sm natural metadata line"
    );
    assert!(field.size.height >= label.size.height && field.size.height >= value.size.height);
    assert!(value.right() <= field.right());
    assert_eq!(
        main.read_with(&window, |main, _| main.download_detail_tab),
        DownloadDetailTab::Overview
    );
}

struct ProgressFixture;
impl Render for ProgressFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(400.0)).flex().flex_col().gap_2().children(
            [0.0, 0.375, 1.5]
                .into_iter()
                .enumerate()
                .map(|(index, fraction)| {
                    download_aggregate_progress(index, fraction, Theme::zed_dark().accent)
                        .into_any_element()
                }),
        )
    }
}
#[gpui_kit::test]
fn queue03_production_progress_paints_clamped_percentage(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, window) = cx.add_window_view(move |_, _| ProgressFixture);
    window
        .refresh()
        .expect("render production Progress semantics");
    let (quads, scale) = window.update(|window, _| (window.painted_quads(), window.scale_factor()));
    let widths: Vec<_> = quads
        .iter()
        .filter(|quad| quad.bounds.size.height.as_f32() / scale == 4.0)
        .map(|quad| quad.bounds.size.width.as_f32() / scale)
        .collect();
    assert!(
        widths.contains(&150.0),
        "37.5 percent of400px: {widths:?}; quads={quads:?}"
    );
    assert!(
        widths.iter().filter(|width| **width == 400.0).count() >= 4,
        "three full tracks plus clamped100percent indicator: {widths:?}"
    );
}

#[gpui_kit::test]
fn queue03_toolbar_roves_focus_and_five_tabs_keep_original_scroll_owner(cx: &mut TestAppContext) {
    let (main, handle, _temp, _downloads) = workspace(cx);
    let mut window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.update(|window, _| window.activate_window());
    window.run_until_parked();
    window.refresh().expect("production keyboard workspace");
    let input = main.read_with(&window, |main, _| main.download_search_input.clone());
    window.update(|window, cx| {
        let focus = input.read(cx).focus_handle(cx);
        focus.focus(window, cx);
    });
    window.update(|window, cx| window.focus_next(cx));
    let first = window
        .update(|window, cx| window.focused(cx))
        .expect("first toolbar focus");
    window.simulate_keystrokes("right");
    let second = window
        .update(|window, cx| window.focused(cx))
        .expect("next toolbar focus");
    assert_ne!(first, second, "official Toolbar arrow moves focus");
    window.simulate_keystrokes("right right");
    assert_eq!(
        window.update(|window, cx| window.focused(cx)),
        Some(first),
        "three-command Toolbar wraps without escaping its subtree"
    );
    for tab in DownloadDetailTab::ALL {
        if tab != DownloadDetailTab::Overview {
            main.update(&mut window, |main, _| {
                main.download_detail_scroll
                    .set_offset(point(px(0.0), px(-40.0)))
            });
        }
        let selector: &'static str =
            Box::leak(format!("download-detail-tab-{}", tab.diagnostic_name()).into_boxed_str());
        let bounds = window.debug_bounds(selector).expect("real official tab");
        window.simulate_click(bounds.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            main.read_with(&window, |main, _| main.download_detail_tab),
            tab
        );
        assert_eq!(
            main.read_with(&window, |main, _| main.download_detail_scroll.offset()),
            point(px(0.0), px(0.0))
        );
        window.refresh().expect("selected tab content");
        let dock = window
            .debug_bounds("download-task-detail")
            .expect("original detail dock");
        assert!(dock.bottom() <= px(480.0));
    }
}

#[gpui_kit::test]
fn queue03_datatable_public_sort_and_context_hooks_are_callable(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let calls = Rc::new(RefCell::new(HashSet::new()));
    let retained = calls.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let state = cx.new(|cx| {
            TableState::new(
                GridDelegate {
                    external: Some(retained),
                },
                window,
                cx,
            )
            .row_selectable(false)
        });
        let view = cx.new(|_| GridFixture { state });
        gpui_kit::base::Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(400.0), px(260.0)));
    window.refresh().expect("sortable public delegate");
    let header = window
        .debug_bounds("candidate-header")
        .expect("actual released header");
    window.simulate_click(point(px(184.0), header.center().y), Modifiers::default());
    assert!(
        calls.borrow().contains(&100),
        "actual released sort icon invokes perform_sort"
    );
    let row = window
        .debug_bounds("candidate-row-1")
        .expect("actual context row");
    window.simulate_mouse_down(
        row.center(),
        gpui_kit::MouseButton::Right,
        Modifiers::default(),
    );
    window.simulate_mouse_up(
        row.center(),
        gpui_kit::MouseButton::Right,
        Modifiers::default(),
    );
    window.run_until_parked();
    window.refresh().expect("released context menu");
    assert!(
        calls.borrow().contains(&201),
        "real secondary gesture invokes public row context_menu"
    );
}

#[gpui_kit::test]
fn queue03_batch_toolbar_buttons_fit_original_rail_at_minimum(cx: &mut TestAppContext) {
    let (main, handle, _temp, downloads) = workspace(cx);
    let mut window = VisualTestContext::from_window(handle, cx);
    let ids = downloads.read_with(&window, |app, _| {
        app.tasks()
            .iter()
            .map(|task| task.task.id())
            .collect::<HashSet<_>>()
    });
    main.update(&mut window, |main, _| {
        main.selected_download_tasks = ids;
    });
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("actual batch toolbar");
    let rail = window
        .debug_bounds("download-batch-toolbar")
        .expect("original batch rail");
    assert_eq!(rail.size.height, px(DOWNLOAD_BATCH_TOOLBAR_HEIGHT));
    let commands: Vec<_> = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .filter(|node| {
            node.role() == Some(gpui_kit::Role::Button)
                && node
                    .path()
                    .contains(&gpui_kit::ElementId::from("download-batch-kit-toolbar"))
        })
        .collect();
    assert!(!commands.is_empty());
    for command in commands {
        let bounds = command.bounds();
        assert!(
            bounds.left() >= rail.left()
                && bounds.top() >= rail.top()
                && bounds.right() <= rail.right()
                && bounds.bottom() <= rail.bottom()
        );
    }
    assert_eq!(downloads.read_with(&window, |app, _| app.tasks().len()), 2);
}

#[gpui_kit::test]
fn installed_feedback_download_filters_and_sort_menu_keep_readable_width(cx: &mut TestAppContext) {
    let (_main, handle, _temp, _downloads) = workspace(cx);
    let mut window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("full filter strip");
    for filter in DownloadTaskFilter::ALL {
        let bounds = window.debug_bounds(filter.id()).expect("filter button");
        assert!(
            bounds.size.width >= px(95.0),
            "full Chinese filter and count require room: {bounds:?}"
        );
        assert!(bounds.right() <= px(920.0));
    }
    let trigger = window.debug_bounds("download-sort").expect("sort trigger");
    window.simulate_click(trigger.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("real sort popup");
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let menu = nodes
        .iter()
        .find(|n| n.role() == Some(gpui_kit::Role::Menu))
        .expect("real menu");
    assert!(
        menu.bounds().size.width >= px(160.0) && menu.bounds().size.width <= px(182.0),
        "bounded readable sort popup: {:?}",
        menu.bounds()
    );
}

#[gpui_kit::test]
fn ux_download_extensions_follow_normal_actions_and_expand_direct_submenu(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;
    cx.update(gpui_kit::init);
    let temp = tempfile::tempdir().expect("owned menu package");
    std::fs::write(temp.path().join("fixture.exe"), b"identity only").expect("fixture");
    std::fs::write(temp.path().join("extension.json"),r#"{"schema_version":1,"id":"fixture.menu","name":"Fixture","version":"1","runtime":{"program":"exe"},"files":["fixture.exe"],"handlers":[{"id":"copy","title":"生成下载清单","feature":"downloads","entry":"action","action":{"type":"script","entry_file":"fixture.exe"}}]}"#.as_bytes()).expect("manifest");
    let preview = nxtools_extensions::ImportPreview::from_directory(temp.path())
        .expect("preview")
        .stage_into(&temp.path().join("staged"))
        .expect("stage");
    let mut config = nxtools_core::AppConfig::default();
    config
        .extensions
        .install(preview, "owned".into(), false)
        .expect("install");
    let package = &mut config.extensions.packages[0];
    package.enabled = true;
    package.trust = Some(nxtools_extensions::TrustApproval {
        content_hash: package.content_hash.clone(),
        runtime: nxtools_extensions::RuntimeIdentity::inspect(
            &package
                .code_directory
                .as_ref()
                .expect("code")
                .join("fixture.exe"),
        )
        .expect("identity"),
    });
    let owner = cx.new(|_| {
        crate::extensions::ExtensionApplication::new(
            config,
            Default::default(),
            None,
            gpui_kit::WeakEntity::new_invalid(),
        )
    });
    cx.update(|cx| {
        cx.set_global(crate::extensions::ExtensionApplicationGlobal(
            owner.downgrade(),
        ))
    });
    let notifications = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&notifications, cx));
    let task_id = DownloadTaskId::new();
    let menu_slot = Rc::new(RefCell::new(None));
    let retained = menu_slot.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let menu = PopupMenu::build(window, cx, |menu, window, cx| {
            download_task_context_menu(
                Theme::zed_dark(),
                menu,
                gpui_kit::WeakEntity::new_invalid(),
                vec![task_id],
                DownloadContextCapabilities {
                    single_artifact: Some(task_id),
                    can_remove: true,
                    ..Default::default()
                },
                window,
                cx,
            )
        });
        *retained.borrow_mut() = Some(menu.clone());
        gpui_kit::base::Root::new(menu, window, cx)
    });
    window.simulate_resize(size(px(900.0), px(620.0)));
    window.update(|window, cx| window.draw(cx).clear(cx));
    let menu = menu_slot.borrow().clone().expect("real menu");
    window.update(|window, cx| menu.read(cx).focus_handle(cx).focus(window, cx));
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let extension = nodes
        .iter()
        .find(|node| node.label() == Some("扩展动作"))
        .expect("extension submenu")
        .bounds();
    let normal = window
        .debug_bounds("download-menu-task-window")
        .expect("normal task action");
    let remove = window
        .debug_bounds("download-menu-remove")
        .expect("danger action");
    assert!(
        normal.bottom() <= extension.top() && extension.bottom() <= remove.top(),
        "normal actions -> extensions -> destructive"
    );
    window.simulate_mouse_move(extension.center(), None, Default::default());
    window.simulate_keystrokes("right");
    window.update(|window, cx| window.draw(cx).clear(cx));
    let leaf = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("生成下载清单"))
            .expect("direct leaf")
            .bounds()
    });
    window.simulate_click(leaf.center(), Default::default());
    window.run_until_parked();
    assert!(
        window.debug_bounds("extension-manual-choices").is_none(),
        "no intermediary chooser"
    );
    assert!(
        !window.update(|window, cx| window.has_active_dialog(cx)),
        "start failure must not open a separate report dialog"
    );
    assert!(
        notifications.read_with(window, |center, _| center.items().iter().any(|item| item
            .level
            == crate::notifications::NotificationLevel::Error
            && item.title == "未能启动扩展动作")),
        "unavailable service reports through the shared notification center without executing fixture"
    );
    assert!(owner.read_with(window, |app, _| app.recent_manual_runs().is_empty()));
}

struct FollowupMetadataFixture;
impl Render for FollowupMetadataFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::zed_dark();
        div()
            .w(px(360.0))
            .flex()
            .flex_col()
            .gap_2()
            .child(download_detail_section(
                theme,
                "请求",
                vec![(
                    "X-Synthetic-Long-Header-Name".repeat(4),
                    "https://synthetic.invalid/long/path?value=retained&next=unchanged".repeat(6),
                )],
            ))
            .child(download_detail_line_with_color(
                theme,
                "错误",
                "原始错误内容".into(),
                theme.danger,
            ))
            .child(
                div()
                    .debug_selector(|| "followup-released-description-line".into())
                    .child(
                        DescriptionList::horizontal()
                            .columns(1)
                            .small()
                            .bordered(false)
                            .label_width(px(DOWNLOAD_DETAIL_LABEL_WIDTH + 8.0))
                            .item(
                                "错误",
                                div().text_sm().child("原始错误内容").into_any_element(),
                                1,
                            ),
                    ),
            )
    }
}

#[gpui_kit::test]
fn followup_download_metadata_uses_released_description_slots_and_wraps(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, window) = cx.add_window_view(|_, _| FollowupMetadataFixture);
    window.simulate_resize(size(px(360.0), px(800.0)));
    window.refresh().expect("production read-only metadata");
    let label = window
        .debug_bounds("download-detail-owned-label")
        .expect("official label slot");
    let value = window
        .debug_bounds("download-detail-owned-value")
        .expect("official value slot");
    let released = window
        .debug_bounds("followup-released-description-line")
        .expect("actual released DescriptionList reference");
    assert!(
        label.size.height > released.size.height,
        "long header name must wrap, not truncate"
    );
    assert!(
        value.size.height > released.size.height,
        "long URL must remain multiline"
    );
    assert!(label.right() <= value.left() && value.right() <= px(360.0));
    let error = window
        .debug_bounds("download-task-detail-error")
        .expect("original error boundary");
    assert_eq!(
        error.size.height, released.size.height,
        "production line must retain the released DescriptionList typography"
    );
}

struct FollowupTimelineFixture {
    detail: DownloadTaskDetail,
    owner: Entity<MainWindow>,
    scroll: ScrollHandle,
}
impl Render for FollowupTimelineFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(download_task_detail(
                Theme::zed_dark(),
                DownloadDetailView {
                    task: &self.detail.task,
                    detail: Some(&self.detail),
                    detail_error: None,
                    progress: None,
                    active_tab: DownloadDetailTab::Timeline,
                    scroll: self.scroll.clone(),
                    main_window: self.owner.clone(),
                },
            ))
            .child(
                Marker::new().content(
                    MarkerContent::new().child(
                        div()
                            .debug_selector(|| "followup-released-marker-message".into())
                            .child(download_timeline_label(&self.detail.timeline[0])),
                    ),
                ),
            )
    }
}

#[gpui_kit::test]
fn followup_download_timeline_markers_keep_order_content_and_original_scroll(
    cx: &mut TestAppContext,
) {
    let (owner, _, _temp, downloads) = workspace(cx);
    let task = downloads.read_with(cx, |app, _| app.tasks()[0].clone());
    let timeline: Vec<_> = (0_u32..24)
        .map(|index| DownloadTaskTimelineEntry {
            sequence: u64::from(index),
            occurred_at_unix_ms: 1_700_000_000_000 + i64::from(index) * 1000,
            event: DownloadTaskTimelineEvent::TransferPlanChanged {
                segment_count: index + 1,
            },
        })
        .collect();
    let expected = timeline.clone();
    let scroll = ScrollHandle::new();
    let retained = scroll.clone();
    let (_, window) = cx.add_window_view(move |_, _| FollowupTimelineFixture {
        detail: DownloadTaskDetail {
            task,
            segments: Vec::new(),
            timeline,
        },
        owner,
        scroll,
    });
    window.simulate_resize(size(px(744.0), px(480.0)));
    window.refresh().expect("production timeline dock");
    let viewport = window
        .debug_bounds("download-detail-scroll")
        .expect("original scroll viewport");
    let first = window
        .debug_bounds("download-detail-marker-0")
        .expect("first Marker");
    let second = window
        .debug_bounds("download-detail-marker-1")
        .expect("second Marker");
    let message = window
        .debug_bounds("download-detail-marker-message-0")
        .expect("MarkerContent");
    let timestamp = window
        .debug_bounds("download-detail-marker-time-0")
        .expect("timestamp content slot");
    let released = window
        .debug_bounds("followup-released-marker-message")
        .expect("actual released MarkerContent reference");
    assert!(first.bottom() <= second.top() && first.left() == second.left());
    assert!(timestamp.right() <= message.left() && message.right() <= viewport.right());
    assert_eq!(
        message.size.height, released.size.height,
        "production timeline must retain the released Marker typography"
    );
    let label = download_timeline_label(&expected[0]);
    assert_eq!(label, "传输计划更新 · 1 个分段");
    for _ in 0..40 {
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position: viewport.center(),
            delta: gpui_kit::ScrollDelta::Lines(point(0.0, -3.0)),
            ..Default::default()
        });
        window.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
    window.refresh().expect("scroll to retained final event");
    let last = window
        .debug_bounds("download-detail-marker-23")
        .expect("last original event");
    assert!(
        last.bottom() <= viewport.bottom() && last.bottom() > viewport.top(),
        "last={last:?}, viewport={viewport:?}, offset={:?}",
        retained.offset()
    );
    assert!(retained.offset().y < px(0.0));
    // Scrollbar/scroll-layout callbacks are one-shot work from the preceding wheel input.
    // Drain them before checking that the static Marker timeline does not keep requesting frames.
    for _ in 0..6 {
        window.executor().advance_clock(Duration::from_millis(100));
        window.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });
        window.run_until_parked();
    }
    assert_eq!(
        window.update(|window, cx| window.simulate_next_frame(cx)),
        0,
        "static timeline has no timer"
    );
}

fn sort_keyboard_workspace(
    cx: &mut TestAppContext,
) -> (
    Entity<MainWindow>,
    gpui_kit::AnyWindowHandle,
    tempfile::TempDir,
    Entity<DownloadApplication>,
) {
    let (main, handle, temp, downloads) = workspace(cx);
    nxtools_core::AppConfig::default()
        .save_to(&temp.path().join("config.json"))
        .expect("seed isolated sort config");
    handle
        .update(cx, |_, window, cx| {
            let content = main.clone();
            window.replace_root(cx, |window, cx| {
                gpui_kit::base::Root::new(content, window, cx)
            });
            window.activate_window();
        })
        .expect("original MainWindow under official Root");
    (main, handle, temp, downloads)
}

fn sort_header_button(
    surface: &mut VisualTestContext,
    key: DownloadSortKey,
) -> gpui_kit::base::test_support::ElementSnapshot {
    surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::Button)
                && node.path().last()
                    == Some(&gpui_kit::ElementId::from((
                        "download-sort-header-button",
                        key as usize,
                    )))
        })
        .unwrap_or_else(|| {
            panic!(
                "actual semantic sortable header Button {}",
                key.diagnostic_name()
            )
        })
}

fn tab_to_sort_header(
    surface: &mut VisualTestContext,
    main: &Entity<MainWindow>,
    key: DownloadSortKey,
) {
    let path = sort_header_button(surface, key).path().to_vec();
    let search = main.read_with(surface, |main, _| main.download_search_input.clone());
    surface.update(|window, cx| search.focus_handle(cx).focus(window, cx));
    for _ in 0..60 {
        surface.simulate_keystrokes("tab");
        surface.refresh().expect("actual header Tab frame");
        if surface
            .update(|window, _| gpui_kit::base::test_support::has_observed_focus(window, &path))
        {
            return;
        }
    }
    panic!(
        "sort header {} not reachable through original Tab order",
        key.diagnostic_name()
    );
}

fn sort_key_event(surface: &mut VisualTestContext, key: &str, down: bool, held: bool) {
    let keystroke = gpui_kit::Keystroke::parse(key).expect("sort activation key");
    if down {
        surface.simulate_event(gpui_kit::KeyDownEvent {
            keystroke,
            is_held: held,
            prefer_character_input: false,
        });
    } else {
        surface.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    }
}

#[gpui_kit::test]
fn sort_keyboard_actual_headers_have_names_tab_focus_and_original_column_bounds(
    cx: &mut TestAppContext,
) {
    let (main, handle, _temp, _downloads) = sort_keyboard_workspace(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    for dimensions in [(1060.0, 650.0), (920.0, 480.0)] {
        surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        surface
            .refresh()
            .expect("actual default/minimum sortable table");
        let header = surface
            .debug_bounds("download-table-header")
            .expect("original header");
        assert_eq!(header.size.height, px(DOWNLOAD_TABLE_HEADER_HEIGHT));
        for (key, selector) in [
            (DownloadSortKey::Name, "download-sort-header-name"),
            (DownloadSortKey::Progress, "download-sort-header-progress"),
            (DownloadSortKey::Status, "download-sort-header-status"),
            (DownloadSortKey::Created, "download-sort-header-created"),
        ] {
            let slot = surface
                .debug_bounds(selector)
                .expect("original column slot");
            let button = sort_header_button(&mut surface, key);
            assert!(
                button
                    .label()
                    .is_some_and(|name| name.contains(key.label()) && name.contains("排序"))
            );
            assert!(
                button.bounds().left() >= slot.left() && button.bounds().right() <= slot.right()
            );
            assert!(
                button.bounds().top() >= header.top()
                    && button.bounds().bottom() <= header.bottom()
            );
            if key == DownloadSortKey::Created {
                assert_eq!(slot.size.width, px(DOWNLOAD_CREATED_COLUMN_WIDTH));
            }
            tab_to_sort_header(&mut surface, &main, key);
            let focus =
                surface.update(|window, cx| window.focused(cx).expect("actual focused header"));
            assert!(surface.update(|window, cx| focus.is_focused(window)
                && window.last_input_was_keyboard()
                && window.focused(cx).is_some()));
            surface.update(|window, cx| window.draw(cx).clear(cx));
            surface.update(|window, cx| {
                let scale = window.scale_factor();
                let expected = gpui_kit::component::Theme::global(cx).colors.ring;
                assert!(
                    window.painted_quads().iter().any(|quad| {
                        (quad.bounds.origin.x.0 / scale - button.bounds().left().as_f32()).abs()
                            < 1.0
                            && (quad.bounds.origin.y.0 / scale - button.bounds().top().as_f32())
                                .abs()
                                < 1.0
                            && (quad.bounds.size.width.0 / scale
                                - button.bounds().size.width.as_f32())
                            .abs()
                                < 1.0
                            && (quad.bounds.size.height.0 / scale
                                - button.bounds().size.height.as_f32())
                            .abs()
                                < 1.0
                            && quad.border_widths.top.0 > 0.0
                            && quad.border_color == expected
                    }),
                    "official focused Button paints its own visible edge inside original column"
                );
            });
        }
    }
}

#[gpui_kit::test]
fn sort_keyboard_enter_space_match_pointer_reverse_once_and_preserve_task_identity(
    cx: &mut TestAppContext,
) {
    let (main, handle, _temp, downloads) = sort_keyboard_workspace(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    surface.refresh().expect("original sort workspace");
    let tasks = downloads.read_with(&surface, |app, _| app.tasks().to_vec());
    let row_selectors = tasks
        .iter()
        .map(|task| {
            let selector: &'static str = Box::leak(
                format!("download-task-row-{}", task.task.id().as_uuid()).into_boxed_str(),
            );
            (task.task.id(), selector)
        })
        .collect::<Vec<_>>();
    main.update_in(&mut surface, |main, _, cx| {
        main.selected_download_tasks
            .extend(tasks.iter().map(|task| task.task.id()));
        main.download_selection_anchor = main.selected_download_task;
        cx.notify();
    });
    let selection = main.read_with(&surface, |main, _| {
        (
            main.selected_download_task,
            main.selected_download_tasks.clone(),
            main.download_selection_anchor,
        )
    });
    for key in [
        DownloadSortKey::Name,
        DownloadSortKey::Progress,
        DownloadSortKey::Status,
        DownloadSortKey::Created,
    ] {
        tab_to_sort_header(&mut surface, &main, key);
        let before = main.read_with(&surface, |main, _| main.download_sort);
        sort_key_event(&mut surface, "enter", true, false);
        sort_key_event(&mut surface, "enter", true, true);
        assert_eq!(
            main.read_with(&surface, |main, _| main.download_sort),
            before,
            "one activation waits for release"
        );
        sort_key_event(&mut surface, "enter", false, false);
        let expected = DownloadSortState {
            key,
            descending: key.default_descending(),
        };
        assert_eq!(
            main.read_with(&surface, |main, _| main.download_sort),
            expected
        );
        sort_key_event(&mut surface, "enter", false, false);
        assert_eq!(
            main.read_with(&surface, |main, _| main.download_sort),
            expected,
            "duplicate release cannot reverse again"
        );
        surface.refresh().expect("actual new-key state");
        let name = sort_header_button(&mut surface, key)
            .label()
            .expect("readable direction")
            .to_owned();
        assert!(name.contains(if expected.descending {
            "降序"
        } else {
            "升序"
        }));
        sort_key_event(&mut surface, "space", true, false);
        sort_key_event(&mut surface, "space", false, false);
        let reversed = DownloadSortState {
            key,
            descending: !expected.descending,
        };
        assert_eq!(
            main.read_with(&surface, |main, _| main.download_sort),
            reversed
        );
        surface.refresh().expect("actual reversed state");
        let button = sort_header_button(&mut surface, key);
        surface.simulate_click(button.bounds().center(), Modifiers::default());
        assert_eq!(
            main.read_with(&surface, |main, _| main.download_sort),
            expected,
            "pointer shares original toggle owner"
        );
        surface.run_until_parked();
        surface.refresh().expect("actual sorted virtual rows");
        let mut mounted = row_selectors
            .iter()
            .map(|(id, selector)| {
                let row = surface
                    .debug_bounds(selector)
                    .expect("actual sorted production row bounds");
                (row.top(), *id)
            })
            .collect::<Vec<_>>();
        mounted.sort_by(|left, right| left.0.as_f32().total_cmp(&right.0.as_f32()));
        let expected_ids = download_tasks_for_view(&tasks, DownloadTaskFilter::All, "", expected)
            .iter()
            .map(|index| tasks[*index].task.id())
            .collect::<Vec<_>>();
        assert_eq!(
            mounted.into_iter().map(|(_, id)| id).collect::<Vec<_>>(),
            expected_ids
        );
        assert_eq!(
            main.read_with(&surface, |main, _| (
                main.selected_download_task,
                main.selected_download_tasks.clone(),
                main.download_selection_anchor
            )),
            selection
        );
        assert_eq!(
            downloads.read_with(&surface, |app, _| app.tasks().to_vec()),
            tasks,
            "sort never mutates authoritative task or queue ordering"
        );
    }
}

#[gpui_kit::test]
fn sort_keyboard_persists_both_fields_and_reopened_main_restores_direction(
    cx: &mut TestAppContext,
) {
    let (main, handle, temp, _downloads) = sort_keyboard_workspace(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    surface.refresh().expect("actual persisted sort workspace");
    tab_to_sort_header(&mut surface, &main, DownloadSortKey::Created);
    for key in ["enter", "space"] {
        sort_key_event(&mut surface, key, true, false);
        sort_key_event(&mut surface, key, false, false);
    }
    surface.run_until_parked();
    let saved = nxtools_core::AppConfig::load_from(&temp.path().join("config.json"))
        .expect("original config section persisted");
    assert_eq!(saved.download.task_list_sort.as_deref(), Some("created"));
    assert_eq!(saved.download.task_list_sort_descending, Some(false));
    let expected = main.read_with(&surface, |main, _| main.download_sort);
    // Focus transfer retires the original Button's armed release.
    sort_key_event(&mut surface, "enter", true, false);
    let search = main.read_with(&surface, |main, _| main.download_search_input.clone());
    surface.update(|window, cx| search.focus_handle(cx).focus(window, cx));
    sort_key_event(&mut surface, "enter", false, false);
    assert_eq!(
        main.read_with(&surface, |main, _| main.download_sort),
        expected
    );
    let notifications = surface.read(|cx| main.read(cx).notifications.clone());
    let restored_notifications = notifications.clone();
    let paths = temp.path().to_path_buf();
    let restored_downloads = surface.update(|_, cx| {
        let (platform, _) = nxtools_ipc::platform_bridge(16);
        cx.new(|cx| {
            DownloadApplication::new(
                nxtools_download::DownloadEngine::open_in_memory()
                    .expect("fresh in-memory reopened engine"),
                Rc::new(platform),
                restored_notifications,
                crate::downloads::DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config: saved,
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: paths.join("downloads.sqlite3"),
                    ffmpeg_install_directory: paths.join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let app = restored_downloads.downgrade();
    let (reopened, second) = surface.add_window_view(move |window, cx| {
        let mut main = MainWindow::new(
            gpui_kit::WeakEntity::new_invalid(),
            gpui_kit::WeakEntity::new_invalid(),
            app,
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main.shell.activate(MainModule::Downloads);
        main
    });
    second
        .refresh()
        .expect("new production MainWindow restores preference");
    assert_eq!(
        reopened.read_with(second, |main, _| main.download_sort),
        expected
    );
}

#[gpui_kit::test]
fn audit06_download_rows_and_header_follow_root_rem(cx: &mut TestAppContext) {
    use gpui_kit::component::Theme as ComponentTheme;
    let (main, handle, _temp, downloads) = sort_keyboard_workspace(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    let tasks = downloads.read_with(&surface, |app, _| app.tasks().to_vec());
    let selectors = tasks
        .iter()
        .map(|task| {
            let selector: &'static str = Box::leak(
                format!("download-task-row-{}", task.task.id().as_uuid()).into_boxed_str(),
            );
            (task.task.id(), selector)
        })
        .collect::<Vec<_>>();
    let mut measured = Vec::new();
    for dimensions in [(1060.0, 650.0), (920.0, 480.0)] {
        surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        for base in [16.0, 14.0, 18.0] {
            surface.update(|window, cx| {
                ComponentTheme::update(cx, |theme| theme.font_size = px(base));
                window.refresh();
            });
            AUDIT06_TYPE_PAINT.with(|paint| paint.borrow_mut().clear());
            surface
                .refresh()
                .expect("actual MainWindow under production Root");
            let rows = selectors
                .iter()
                .map(|(_, selector)| surface.debug_bounds(selector).expect("actual retained row"))
                .collect::<Vec<_>>();
            let name = surface
                .debug_bounds("download-task-name-text-selected")
                .expect("actual primary line");
            let path = surface
                .debug_bounds("download-task-size-text-selected")
                .expect("actual secondary line");
            let category = surface
                .debug_bounds("download-task-category-selected")
                .expect("category cell");
            let category_header = surface
                .debug_bounds("download-category-header")
                .expect("matching header cell");
            let header = surface
                .debug_bounds("download-table-header")
                .expect("header");
            let vector = surface
                .debug_bounds("download-task-icon-fallback-selected")
                .expect("actual vector fallback");
            let rem = surface.update(|window, _| window.rem_size());
            let painted = surface.update(|window, _| {
                let selected = crate::theme::Theme::zed_dark().element_selected;
                window.painted_quads().iter().any(|quad| {
                    (quad.bounds.size.height.as_f32() / window.scale_factor()
                        - rows[0].size.height.as_f32())
                    .abs()
                        <= 0.1
                        && quad.bounds.size.width.as_f32() / window.scale_factor() > 300.0
                        && quad.background == gpui_kit::Background::from(selected)
                })
            });
            let types = AUDIT06_TYPE_PAINT.with(|paint| paint.borrow().clone());
            measured.push((
                dimensions,
                base,
                rem,
                rows,
                name,
                path,
                category,
                category_header,
                header,
                vector,
                painted,
                types,
            ));
        }
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
    assert_eq!(selectors.len(), 2);
    for (
        dimensions,
        base,
        rem,
        rows,
        name,
        path,
        category,
        category_header,
        header,
        vector,
        painted,
        types,
    ) in measured
    {
        let ratio = base / 16.0;
        assert_eq!(rem, px(base));
        assert_eq!(
            rows[0].size.height,
            px(48.0 * ratio),
            "real virtual row height at base={base}"
        );
        assert_eq!(
            (rows[1].top() - rows[0].top()).abs(),
            px(48.0 * ratio),
            "uniform_list measurement uses actual row role"
        );
        assert!((name.size.height - px(20.0 * ratio)).abs() <= px(0.5));
        assert!((path.size.height - px(18.0 * ratio)).abs() <= px(0.5));
        assert!((path.top() - name.bottom() - px(4.0 * ratio)).abs() <= px(0.5));
        assert_eq!(category.size.width, px(64.0 * ratio));
        assert_eq!(category_header.size.width, category.size.width);
        assert!(
            (category.left() - category_header.left()).abs() <= px(4.0),
            "existing fixed selection edges remain"
        );
        assert_eq!(header.size.height, px(28.0 * ratio));
        assert!((vector.size.height - px(30.0 * ratio)).abs() <= px(0.5));
        assert!(rows.iter().all(|row| row.right() <= px(dimensions.0)));
        assert!(
            painted,
            "actual selected row quad must retain scaled row height"
        );
        assert_eq!(
            types.get("name"),
            Some(&(px(13.0 * ratio), px(20.0 * ratio))),
            "actual primary text paint style"
        );
        assert_eq!(
            types.get("path"),
            Some(&(px(12.0 * ratio), px(18.0 * ratio))),
            "actual supporting text paint style"
        );
    }
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_tasks.len()),
        1
    );
}

thread_local! {
    static AUDIT06_TYPE_PAINT: RefCell<HashMap<&'static str, (gpui_kit::Pixels, gpui_kit::Pixels)>> = RefCell::new(HashMap::new());
}

pub(super) fn audit06_type_probe(role: &'static str) -> AnyElement {
    gpui_kit::canvas(
        |_, _, _| (),
        move |_, (), window, _| {
            let style = window.text_style();
            let font = style.font_size.to_pixels(window.rem_size());
            let line = style
                .line_height
                .to_pixels(style.font_size, window.rem_size());
            AUDIT06_TYPE_PAINT.with(|paint| paint.borrow_mut().insert(role, (font, line)));
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

#[gpui_kit::test]
fn audit06_virtual_scroll_remeasures_and_selects_current_stable_id(cx: &mut TestAppContext) {
    use gpui_kit::component::Theme as ComponentTheme;
    let (main, handle, _temp, downloads) = sort_keyboard_workspace(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    downloads.update(&mut surface, |app, cx| {
        let request = app.tasks()[0].task.request().clone();
        for index in 0..28 {
            let mut request = request.clone();
            request.source_url = format!("https://synthetic.invalid/queued-{index}");
            request.file_name = Some(format!("合成任务-{index:02}.bin"));
            app.submit_with_idempotency(
                request,
                nxtools_download::DownloadSubmitMode::QueueOnly,
                None,
                cx,
            )
            .expect("queue only, never execute transfer");
        }
    });
    let ordered = surface.read(|cx| {
        let view = main.read(cx);
        download_tasks_for_view(
            downloads.read(cx).tasks(),
            view.download_filter,
            "",
            view.download_sort,
        )
        .into_iter()
        .map(|index| downloads.read(cx).tasks()[index].task.id())
        .collect::<Vec<_>>()
    });
    let last_id = *ordered.last().expect("last immutable task");
    let selector: &'static str =
        Box::leak(format!("download-task-row-{}", last_id.as_uuid()).into_boxed_str());
    let mut measured = Vec::new();
    for dimensions in [(1060.0, 650.0), (920.0, 480.0)] {
        surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        for base in [16.0, 14.0, 18.0] {
            surface.update(|window, cx| {
                ComponentTheme::update(cx, |theme| theme.font_size = px(base));
                main.update(cx, |view, cx| {
                    view.download_list_scroll
                        .scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                });
                window.refresh();
            });
            surface.refresh().expect("new rem remeasures actual rows");
            main.update(&mut surface, |view, cx| {
                view.download_list_scroll.scroll_to_bottom();
                cx.notify();
            });
            surface.refresh().expect("actual uniform-list end");
            let row = surface
                .debug_bounds(selector)
                .expect("last actual virtual row");
            let viewport = surface
                .debug_bounds("download-task-list-viewport")
                .expect("actual list viewport");
            surface.simulate_click(row.center(), Modifiers::default());
            surface.run_until_parked();
            surface
                .refresh()
                .expect("current stable-ID pointer selection");
            let selected = main.read_with(&surface, |view, _| view.selected_download_task);
            let end = main.read_with(&surface, |view, _| {
                view.download_list_scroll.is_scrolled_to_end()
            });
            measured.push((base, row, viewport, selected, end));
        }
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
    assert_eq!(ordered.len(), 30);
    for (base, row, viewport, selected, end) in measured {
        assert_eq!(
            row.size.height,
            px(3.0 * base),
            "actual end-row measurement follows current rem"
        );
        assert!(
            row.top() >= viewport.top() && row.bottom() <= viewport.bottom() + px(0.5),
            "last row fully reachable: row={row:?}, viewport={viewport:?}"
        );
        assert_eq!(end, Some(true));
        assert_eq!(
            selected,
            Some(last_id),
            "pointer after remeasure retains immutable identity"
        );
    }
}

struct Audit06BitmapRow {
    task: nxtools_download::DownloadEngineTask,
    main: Entity<MainWindow>,
    downloads: gpui_kit::WeakEntity<DownloadApplication>,
    icon: std::path::PathBuf,
}
impl Render for Audit06BitmapRow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().flex_col().child(download_task_row(
            Theme::zed_dark(),
            self.downloads.clone(),
            self.main.clone(),
            DownloadRowSelection {
                selected: true,
                visible_task_ids: Arc::new(vec![self.task.task.id()]),
            },
            &self.task,
            None,
            Some(self.icon.clone()),
        ))
    }
}

#[gpui_kit::test]
fn audit06_bitmap_and_physical_selection_edges_remain_fixed(cx: &mut TestAppContext) {
    use gpui_kit::component::Theme as ComponentTheme;
    let (main, handle, temp, downloads) = sort_keyboard_workspace(cx);
    let original = handle
        .update(cx, |_, _, cx| ComponentTheme::global(cx).clone())
        .expect("theme");
    let task = cx.read(|cx| downloads.read(cx).tasks()[0].clone());
    let icon = temp.path().join("synthetic-icon.png");
    image::RgbaImage::from_pixel(20, 20, image::Rgba([12, 34, 56, 255]))
        .save(&icon)
        .expect("temporary synthetic bitmap");
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| Audit06BitmapRow {
            task,
            main,
            downloads: downloads.downgrade(),
            icon,
        });
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(740.0), px(180.0)));
    let mut measured = Vec::new();
    for base in [16.0, 14.0, 18.0] {
        surface.update(|window, cx| {
            ComponentTheme::update(cx, |theme| theme.font_size = px(base));
            window.refresh();
        });
        surface.refresh().expect("production bitmap row");
        let bitmap = surface
            .debug_bounds("download-task-shell-icon-selected")
            .expect("actual bitmap bounds");
        let edges = surface.update(|window, _| {
            window.painted_quads().iter().any(|quad| {
                (quad.bounds.size.height.as_f32() / window.scale_factor() - 3.0 * base).abs() <= 0.1
                    && (quad.border_widths.left.as_f32() / window.scale_factor() - 2.0).abs() <= 0.1
            })
        });
        measured.push((bitmap, edges));
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
    for (bitmap, edges) in measured {
        assert_eq!(
            bitmap.size,
            size(px(40.0), px(40.0)),
            "file image doubles without following text rem"
        );
        assert!(edges, "physical selection edge remains two logical pixels");
    }
}

#[gpui_kit::test]
fn ux_repair_download_single_click_selects_and_menu_explicitly_opens_details(
    cx: &mut TestAppContext,
) {
    let (main, handle, _temp, downloads) = workspace(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(1060.0), px(650.0)));
    main.update(&mut surface, |main, cx| {
        main.retire_download_detail_scope(cx);
        main.selected_download_task = None;
        main.selected_download_tasks.clear();
    });
    surface.refresh().expect("production downloads");
    let task = downloads.read_with(&surface, |app, _| app.tasks()[0].task.id());
    let selector: &'static str =
        Box::leak(format!("download-task-row-{}", task.as_uuid()).into_boxed_str());
    let row = surface.debug_bounds(selector).expect("actual task row");
    surface.simulate_click(row.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("selection");
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(task)
    );
    assert!(
        main.read_with(&surface, |main, _| main.download_detail_scope.is_none()),
        "selection must not admit detail IO"
    );
    assert!(surface.debug_bounds("download-task-detail").is_none());
    surface.simulate_mouse_down(
        row.center(),
        gpui_kit::MouseButton::Right,
        Modifiers::default(),
    );
    surface.run_until_parked();
    surface.refresh().expect("original context menu");
    let action = surface
        .debug_bounds("download-menu-view-details")
        .expect("explicit View Details action");
    let remove = surface
        .debug_bounds("download-menu-remove")
        .expect("original remove action");
    assert!(
        action.top() > remove.bottom(),
        "View Details must follow all original actions"
    );
    surface.simulate_click(action.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("explicit inspector");
    assert!(surface.debug_bounds("download-task-detail").is_some());
    assert_eq!(
        main.read_with(&surface, |main, _| main
            .download_detail_scope
            .as_ref()
            .map(|scope| scope.task_id())),
        Some(task)
    );
    let scope = main.read_with(&surface, |main, _| {
        main.download_detail_scope.clone().expect("explicit scope")
    });
    let task_states = downloads.read_with(&surface, |app, _| {
        app.tasks()
            .iter()
            .map(|entry| (entry.task.id(), entry.task.status()))
            .collect::<Vec<_>>()
    });
    let close = surface
        .debug_bounds("download-detail-close")
        .expect("original inspector close action");
    surface.simulate_click(close.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(!scope.is_live(), "close retires the original detail scope");
    assert!(main.read_with(&surface, |main, _| main.download_detail_scope.is_none()));
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(task),
        "close preserves selection"
    );
    assert_eq!(
        downloads.read_with(&surface, |app, _| app
            .tasks()
            .iter()
            .map(|entry| (entry.task.id(), entry.task.status()))
            .collect::<Vec<_>>()),
        task_states,
        "inspector close must not change task state"
    );
}
