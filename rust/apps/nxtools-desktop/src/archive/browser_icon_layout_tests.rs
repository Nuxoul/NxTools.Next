//! Actual BrowserHost -> released TabBar/Tab and ArchiveBrowser -> released Tree boundaries.
//! Synthetic metadata/IPC and TestWindow only; never a product/native/DPI acceptance claim.
use super::*;
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{
    Bounds, MouseDownEvent, MouseUpEvent, Pixels, Role, ScrollDelta, ScrollEvent, TestAppContext,
    TouchPhase, VisualTestContext, point, size,
};
use std::path::PathBuf;
use std::{collections::BTreeMap, sync::Arc};

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        let cache = cx.new(|_| ApprovedImages::new());
        window.refresh();
        window.with_image_cache(Some(cache.clone().into()), |window| {
            window.draw(cx).clear(cx)
        });
        for path in [
            "brand/archive.png",
            "brand/archive-40.png",
            "brand/nxtools-40.png",
        ] {
            assert!(
                window.has_image_atlas_entry(cache.read(cx).0.get(path).expect("approved image")),
                "actual production img painted approved pixels: {path}"
            );
        }
    });
    surface.run_until_parked();
}

fn mount(
    cx: &mut TestAppContext,
    paths: &[&str],
) -> (Entity<ArchiveApplication>, Vec<Uuid>, VisualTestContext) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    cx.update(crate::theme::apply_component_theme);
    owner.update(cx, |app, cx| {
        for path in paths {
            app.open_canonical(PathBuf::from(path), 0, vec![], cx);
        }
    });
    cx.run_until_parked();
    let (window, ids, views) = owner.read_with(cx, |app, _| {
        let ids = paths
            .iter()
            .map(|path| app.paths[&PathBuf::from(path)])
            .collect::<Vec<_>>();
        let views = ids
            .iter()
            .map(|id| app.windows[id].view.upgrade().expect("live browser"))
            .collect::<Vec<_>>();
        (app.host.as_ref().expect("original host").window, ids, views)
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.simulate_resize(size(px(960.), px(640.)));
    for view in views {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                let entries = ["folder/deep/file.txt", "other.txt"]
                    .into_iter()
                    .enumerate()
                    .map(|(ordinal, name)| {
                        serde_json::from_value(serde_json::json!({
                            "ordinal": ordinal, "name": name, "kind": "File", "size": 3,
                            "packed_size": null, "encrypted": false, "warning_codes": [],
                            "provenance": "Utf8Flag", "codepage": 65001
                        }))
                        .expect("synthetic metadata")
                    })
                    .collect();
                view.install(Uuid::new_v4(), entries, "Zip".into(), window, cx);
            });
        });
    }
    draw(&mut surface);
    (owner, ids, surface)
}

fn bounds(surface: &mut VisualTestContext, selector: &str) -> Bounds<Pixels> {
    surface
        .debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .unwrap_or_else(|| panic!("current-frame {selector}"))
}
fn close(owner: &Entity<ArchiveApplication>, surface: &mut VisualTestContext) {
    let id = owner.read_with(surface, |app, _| app.host.as_ref().expect("host").id);
    owner.update(surface, |app, cx| app.close_host(id, cx));
    surface.run_until_parked();
}

#[gpui_kit::test]
fn icon_layout_official_tab_prefix_has_balanced_edge_insets(cx: &mut TestAppContext) {
    let (owner, ids, mut surface) = mount(cx, &[r"C:\synthetic\short.zip"]);
    let tab = bounds(&mut surface, &format!("archive-tab-{}", ids[0]));
    let icon = bounds(&mut surface, &format!("archive-tab-icon-{}", ids[0]));
    let close_button = bounds(&mut surface, &format!("archive-tab-close-{}", ids[0]));
    println!("official Tab={tab:?} img={icon:?} close={close_button:?}");
    assert_eq!(icon.size, size(px(16.), px(16.)));
    assert_eq!(
        icon.left() - tab.left(),
        px(13.),
        "12DIP inset after official1DIP border; prefix must not touch tab edge"
    );
    assert_eq!(tab.right() - close_button.right(), px(13.));
    assert_eq!(icon.center().y, close_button.center().y);
    close(&owner, &mut surface);
}

#[gpui_kit::test]
fn narrow_tree_folder_name_stays_on_one_line_inside_fixed_row(cx: &mut TestAppContext) {
    let (owner, ids, mut surface) = mount(cx, &[r"C:\synthetic\folders.zip"]);
    let view = owner.read_with(&surface, |app, _| {
        app.windows[&ids[0]].view.upgrade().expect("live browser")
    });
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let entry = serde_json::from_value(serde_json::json!({
                "ordinal":0,"name":"data/flutter_assets/child.txt","kind":"File","size":3,
                "packed_size":null,"encrypted":false,"warning_codes":[],
                "provenance":"Utf8Flag","codepage":65001
            }))
            .expect("synthetic long folder name");
            view.install(Uuid::new_v4(), vec![entry], "Zip".into(), window, cx);
            view.navigate("data/flutter_assets".into(), window, cx);
        });
    });
    draw(&mut surface);
    let label = bounds(&mut surface, "archive-tree-label-data/flutter_assets");
    let icon = bounds(&mut surface, "archive-tree-icon-data/flutter_assets");
    assert!(
        label.size.height <= px(24.),
        "folder label must not wrap under the 32DIP tree row: {label:?}"
    );
    let (narrow_text, short_text) = surface.update(|window, _| {
        let texts = gpui_kit::base::test_support::texts(window);
        let text_bounds = |name| {
            texts
                .iter()
                .find(|text| {
                    text.text() == name
                        && text.path().last()
                            == Some(&gpui_kit::ElementId::from("archive-tree-folder-name"))
                        && text.visible()
                })
                .expect("painted official tree label")
                .bounds()
        };
        (text_bounds("flutter_assets"), text_bounds("data"))
    });
    assert_eq!(
        narrow_text, label,
        "measure the painted label, not a fixed row slot"
    );
    assert!(narrow_text.size.width < short_text.size.width);
    assert_eq!(
        narrow_text.size.height, short_text.size.height,
        "the constrained name uses the same one line as its short parent"
    );
    assert_eq!(icon.center().y, label.center().y);
    close(&owner, &mut surface);
}

#[gpui_kit::test]
fn icon_layout_official_tree_icon_slots_balance_32dip_rows(cx: &mut TestAppContext) {
    let (owner, _, mut surface) = mount(cx, &[r"C:\synthetic\long-root-压缩包.zip"]);
    let root = bounds(&mut surface, "archive-tree-icon-");
    let folder = bounds(&mut surface, "archive-tree-icon-folder");
    let label = bounds(&mut surface, "archive-tree-label-");
    println!("official Tree root={root:?} folder={folder:?} label={label:?}");
    assert_eq!(
        root.size,
        size(px(20.), px(20.)),
        "tree icon must leave6DIP above/below in32DIP row, not24DIP artwork plus ListItem padding"
    );
    assert_eq!(folder.size, root.size);
    assert_eq!(label.left() - root.right(), px(4.));
    assert_eq!(root.center().y, label.center().y);
    close(&owner, &mut surface);
}

#[gpui_kit::test]
fn icon_layout_single_caption_has_approved_blue_app_slot(cx: &mut TestAppContext) {
    let (owner, _, mut surface) = mount(cx, &[r"C:\synthetic\caption.zip"]);
    let caption = bounds(&mut surface, "archive-browser-titlebar");
    assert_eq!(caption.size.height, px(34.));
    let icon = bounds(&mut surface, "archive-caption-app-icon");
    assert_eq!(icon.size, size(px(20.), px(20.)));
    assert_eq!(icon.left() - caption.left(), px(12.));
    assert_eq!(icon.center().y, caption.center().y - px(0.5)); // bottom1DIP caption divider
    let tabs = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|node| node.role() == Some(Role::Tab))
            .count()
    });
    assert_eq!(tabs, 1);
    close(&owner, &mut surface);
}

/// TestApp deliberately has an empty AssetSource. Supply the SAME embedded PNG bytes to the
/// actual production img elements through GPUI's public cache, not a fake image/control.
struct ApprovedImages(BTreeMap<String, Arc<gpui_kit::RenderImage>>);
impl ApprovedImages {
    fn new() -> Self {
        use gpui_kit::AssetSource as _;
        let mut images = BTreeMap::new();
        for path in [
            "brand/nxtools.png",
            "brand/archive.png",
            "brand/nxtools-40.png",
            "brand/archive-40.png",
        ] {
            let bytes = crate::assets::NxToolsAssets
                .load(path)
                .expect("asset load")
                .expect("approved PNG");
            let mut pixels = image::load_from_memory(&bytes)
                .expect("decode approved PNG")
                .to_rgba8();
            for pixel in pixels.pixels_mut() {
                pixel.0.swap(0, 2);
            } // GPUI RenderImage is BGRA
            images.insert(
                path.into(),
                Arc::new(gpui_kit::RenderImage::new(vec![image::Frame::new(pixels)])),
            );
        }
        Self(images)
    }
}
impl gpui_kit::ImageCache for ApprovedImages {
    fn load(
        &mut self,
        resource: &gpui_kit::Resource,
        _: &mut Window,
        _: &mut gpui_kit::App,
    ) -> Option<Result<Arc<gpui_kit::RenderImage>, gpui_kit::ImageCacheError>> {
        match resource {
            gpui_kit::Resource::Embedded(path) => self.0.get(path.as_ref()).cloned().map(Ok),
            _ => None,
        }
    }
}

fn activate(owner: &Entity<ArchiveApplication>, id: Uuid, surface: &mut VisualTestContext) {
    let host = owner.read_with(surface, |app, _| {
        app.host
            .as_ref()
            .expect("host")
            .view
            .upgrade()
            .expect("host entity")
    });
    surface.update(|window, cx| {
        host.update(cx, |host, cx| {
            host.activate(id, window, cx);
        })
    });
    draw(surface);
}

#[gpui_kit::test]
fn icon_layout_official_boundaries_matrix_narrow_long_multi_theme_scale(cx: &mut TestAppContext) {
    let long = format!(r"C:\synthetic\{}.zip", "长文件名-archive-".repeat(18));
    for multi in [false, true] {
        let paths = if multi {
            vec![
                r"C:\first\same.zip",
                r"C:\second\same.zip",
                long.as_str(),
                r"C:\a\four.zip",
                r"C:\a\five.zip",
                r"C:\a\six.zip",
                r"C:\a\seven.zip",
            ]
        } else {
            vec![long.as_str()]
        };
        let (owner, ids, mut surface) = mount(cx, &paths);
        for theme in [ThemeMode::Light, ThemeMode::Dark] {
            surface.update(|window, cx| Theme::change(theme, Some(window), cx));
            for dimensions in [
                size(px(720.), px(480.)),
                size(px(960.), px(640.)),
                size(px(1440.), px(900.)),
            ] {
                surface.simulate_resize(dimensions);
                for scale in [1., 1.25, 1.5, 2.] {
                    surface.simulate_scale_factor_change(scale);
                    for index in [0, ids.len() - 1] {
                        activate(&owner, ids[index], &mut surface);
                        let tab = bounds(&mut surface, &format!("archive-tab-{}", ids[index]));
                        let icon =
                            bounds(&mut surface, &format!("archive-tab-icon-{}", ids[index]));
                        let close_button =
                            bounds(&mut surface, &format!("archive-tab-close-{}", ids[index]));
                        let label_end = bounds(
                            &mut surface,
                            &format!("archive-tab-label-end-{}", ids[index]),
                        );
                        assert_eq!(icon.size, size(px(16.), px(16.)));
                        assert_eq!(close_button.size, size(px(20.), px(20.)));
                        assert!(tab.size.width <= px(220.));
                        assert!(tab.left() >= px(-0.1) && tab.right() <= dimensions.width);
                        near(icon.left() - tab.left(), px(13.), scale);
                        near(tab.right() - close_button.right(), px(13.), scale);
                        near(close_button.left() - label_end.left(), px(8.), scale);
                        assert!(label_end.left() + px(1. / scale) >= icon.right() + px(8.));
                        near(icon.center().y, close_button.center().y, scale);
                        let root = bounds(&mut surface, "archive-tree-icon-");
                        let folder = bounds(&mut surface, "archive-tree-icon-folder");
                        assert_eq!(root.size, size(px(20.), px(20.)));
                        assert_eq!(folder.size, root.size);
                        let label = bounds(&mut surface, "archive-tree-label-");
                        near(label.left() - root.right(), px(4.), scale);
                        near(label.center().y, root.center().y, scale);
                        let caption = bounds(&mut surface, "archive-browser-titlebar");
                        let app_icon = bounds(&mut surface, "archive-caption-app-icon");
                        let window_close = bounds(&mut surface, "archive-browser-window-close");
                        near(caption.size.height, px(34.), scale);
                        assert_eq!(app_icon.size, size(px(20.), px(20.)));
                        assert_eq!(window_close.right(), dimensions.width);
                        assert!(app_icon.right() < window_close.left());
                        // Nonzero alpha rectangle of the ACTUAL32px archive canvas mapped
                        // into the measured img allocation. Never mistake canvas for content.
                        let visible_width = icon.size.width * 28. / 32.;
                        let visible_height = icon.size.height * 24. / 32.;
                        assert_eq!(visible_width, px(14.));
                        assert_eq!(visible_height, px(12.));
                        assert!(visible_width * scale < icon.size.width * scale);
                    }
                }
            }
        }
        println!("matrix multi={multi}:2themes x3viewports x4scales x2selectedpositions passed");
        close(&owner, &mut surface);
    }
}

fn pointer_click(surface: &mut VisualTestContext, target: Bounds<Pixels>) {
    surface.simulate_event(MouseDownEvent {
        position: target.center(),
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position: target.center(),
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    draw(surface);
}

#[gpui_kit::test]
fn icon_layout_same_basename_wheel_menu_pointer_and_keyboard_close_keep_owners(
    cx: &mut TestAppContext,
) {
    let (owner, ids, mut surface) = mount(
        cx,
        &[
            r"C:\one\same.zip",
            r"C:\two\same.zip",
            r"C:\a\three.zip",
            r"C:\a\four.zip",
            r"C:\a\five.zip",
            r"C:\a\six.zip",
            r"C:\a\seven.zip",
        ],
    );
    surface.simulate_resize(size(px(720.), px(480.)));
    activate(&owner, ids[0], &mut surface);
    let host = owner.read_with(&surface, |app, _| {
        app.host
            .as_ref()
            .expect("host")
            .view
            .upgrade()
            .expect("host")
    });
    let before = host.read_with(&surface, |host, _| host.scroll.offset());
    surface.simulate_event(ScrollEvent {
        position: point(px(300.), px(50.)),
        delta: ScrollDelta::Lines(point(0., -100.)),
        touch_phase: TouchPhase::Moved,
        ..Default::default()
    });
    draw(&mut surface);
    let after = host.read_with(&surface, |host, _| host.scroll.offset());
    assert!(
        after.x < before.x,
        "original official vertical-wheel tab scrolling"
    );
    activate(&owner, ids[0], &mut surface);
    let target = bounds(&mut surface, &format!("archive-tab-{}", ids[1]));
    pointer_click(&mut surface, target);
    assert_eq!(
        host.read_with(&surface, |host, _| host.active),
        Some(ids[1])
    );
    assert_eq!(owner.read_with(&surface, |app, _| app.windows.len()), 7);
    let more = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(Role::Button)
                    && node.path().last() == Some(&gpui_kit::ElementId::from("more"))
            })
            .expect("official TabBar overflow button")
            .bounds()
    });
    pointer_click(&mut surface, more);
    let last = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("seven.zip") && node.role() == Some(Role::MenuItem))
            .expect("official full-label menu entry")
            .bounds()
    });
    pointer_click(&mut surface, last);
    assert_eq!(
        host.read_with(&surface, |host, _| host.active),
        Some(ids[6])
    );
    activate(&owner, ids[1], &mut surface);
    // Close still belongs to the real official Button, with its original keyboard release.
    let close_selector = format!("archive-tab-close-{}", ids[1]);
    let target = bounds(&mut surface, &close_selector);
    let mut focused = false;
    for _ in 0..30 {
        surface.simulate_keystrokes("tab");
        draw(&mut surface);
        focused = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.label() == Some("关闭压缩包标签页")
                        && node.bounds() == target
                        && node.focused() == Some(true)
                })
        });
        if focused {
            break;
        }
    }
    assert!(
        focused,
        "official close remains reachable in original Tab order"
    );
    // simulate_keystrokes sends key-down; Base Button intentionally commits on release.
    surface.simulate_keystrokes("space");
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("close release"),
    });
    draw(&mut surface);
    owner.read_with(&surface, |app, _| {
        assert!(!app.windows.contains_key(&ids[1]));
        assert!(app.windows.contains_key(&ids[0]));
    });
    close(&owner, &mut surface);
}

#[test]
fn icon_layout_approved_assets_alpha_and_scale_keep_original_designs() {
    use gpui_kit::AssetSource as _;
    use sha2::{Digest as _, Sha256};
    type OriginalProof<'a> = (&'a [u8], &'a str, [u32; 4], [u32; 4]);
    let originals: [OriginalProof<'_>; 2] = [
        (
            include_bytes!(
                "../../../../../.trellis/tasks/10-16-archive-manager/assets/app-icon-blue-nx-approved.png"
            ),
            "17d3a0f8c423595a62cf991c7d62d01abdb39730681f4d8e7e6401352cd3452f",
            [97, 122, 1206, 1143],
            [105, 124, 1149, 1138],
        ),
        (
            include_bytes!(
                "../../../../../.trellis/tasks/10-16-archive-manager/assets/archive-icon-orange-nx-approved.png"
            ),
            "b0e3da8afb6d5f3bc7061ade4b9ac8344c01f976e26dcc0f13cc4a3a3952565b",
            [47, 0, 1224, 1254],
            [95, 163, 1160, 1089],
        ),
    ];
    for (bytes, hash, nonzero, visible) in originals {
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), hash);
        let image = image::load_from_memory(bytes).expect("original").to_rgba8();
        assert_eq!(image.dimensions(), (1254, 1254));
        assert_eq!(alpha_bounds(&image, 1), nonzero);
        assert_eq!(alpha_bounds(&image, 8), visible);
    }
    for (path, canvas, expected) in [
        ("brand/nxtools.png", 32, [2, 2, 30, 29]),
        ("brand/archive.png", 32, [2, 4, 30, 28]),
        ("brand/nxtools-40.png", 40, [2, 3, 37, 37]),
        ("brand/archive-40.png", 40, [2, 5, 37, 35]),
    ] {
        let bytes = crate::assets::NxToolsAssets
            .load(path)
            .expect("asset")
            .expect("PNG");
        let image = image::load_from_memory(&bytes)
            .expect("asset pixels")
            .to_rgba8();
        assert_eq!(image.dimensions(), (canvas, canvas));
        assert_eq!(alpha_bounds(&image, 1), expected);
        for size in [16., 20.] {
            for scale in [1., 1.25, 1.5, 2.] {
                let device = size * scale;
                let content = [
                    (expected[2] - expected[0]) as f32 * device / canvas as f32,
                    (expected[3] - expected[1]) as f32 * device / canvas as f32,
                ];
                assert!(content[0] < device && content[1] < device);
                assert!(
                    (content[0] / content[1]
                        - (expected[2] - expected[0]) as f32 / (expected[3] - expected[1]) as f32)
                        .abs()
                        < 0.0001
                );
            }
        }
    }
}
fn alpha_bounds(image: &image::RgbaImage, threshold: u8) -> [u32; 4] {
    let mut bounds = [image.width(), image.height(), 0, 0];
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0[3] >= threshold {
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x + 1);
            bounds[3] = bounds[3].max(y + 1);
        }
    }
    bounds
}

fn near(actual: Pixels, expected: Pixels, scale: f32) {
    // Taffy rounds each edge to device pixels; allow at most one physical pixel,
    // not arbitrary layout drift, while preserving exact96/192DPI size assertions.
    assert!(
        (actual - expected).abs() <= px(1. / scale),
        "actual={actual:?} expected={expected:?} scale={scale}"
    );
}

#[gpui_kit::test]
fn icon_layout_tree_icon_and_text_still_enter_original_held_native_drag(cx: &mut TestAppContext) {
    use nxtools_protocol::{
        Message, PlatformCommand,
        archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding},
    };
    for selector in [
        "archive-tree-icon-",
        "archive-tree-label-",
        "archive-tree-icon-folder",
        "archive-tree-label-folder",
    ] {
        let (owner, ids, mut surface) = mount(cx, &[r"C:\synthetic\drag.zip"]);
        let view = owner.read_with(&surface, |app, _| {
            app.windows[&ids[0]].view.upgrade().expect("browser")
        });
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let platform = std::rc::Rc::new(platform);
        let connection = platform
            .connection_generation()
            .expect("original read connection");
        let snapshot = Uuid::new_v4();
        surface.update(|window, cx| view.update(cx, |view, cx| {
            let entries = ["folder/deep/file.txt", "other.txt"].into_iter().enumerate().map(|(ordinal,name)| serde_json::from_value(serde_json::json!({
                "ordinal":ordinal,"name":name,"kind":"File","size":3,"packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001
            })).expect("drag metadata")).collect();
            view.install(snapshot, entries, "Zip".into(), window, cx);
            view.configure_drag(platform, ArchiveBrowserContext {snapshot_id:snapshot,format:"Zip".into(),comment:String::new(),source:ArchiveSourceBinding {native:"v1:owned".into(),digest:[1;32],size:99,modified_unix_ns:1},sevenz_edit:None,tar_edit:None},connection,0,true);
            view.set_drag_hwnd_for_test(917); // only native HWND dependency is synthetic
        }));
        draw(&mut surface);
        let target = bounds(&mut surface, selector);
        surface.simulate_event(MouseDownEvent {
            position: target.center(),
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "down alone never starts drag"
        );
        surface.simulate_event(gpui_kit::MouseMoveEvent {
            position: point(target.center().x + px(12.), target.center().y),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("direct original queue before mouse-up");
        let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(command)) = wire.message
        else {
            panic!("original virtual drag");
        };
        assert_eq!(command.source_window_handle, 917);
        let items = command.items.expect("receiver folder");
        assert_eq!(command.action, "locate");
        assert_eq!(items.len(), 1);
        assert!(items[0].directory && items[0].entry.ordinal.is_none());
        assert_eq!(
            items[0].name,
            if selector.ends_with("folder") {
                "folder"
            } else {
                "drag"
            }
        );
        close(&owner, &mut surface);
    }
}

#[gpui_kit::test]
fn icon_layout_tree_disclosure_remains_separate_from_drag_icon_and_text(cx: &mut TestAppContext) {
    let (owner, ids, mut surface) = mount(cx, &[r"C:\synthetic\tree.zip"]);
    let root = bounds(&mut surface, "archive-tree-icon-");
    let root_arrow = bounds(&mut surface, "archive-tree-disclosure-");
    let folder = bounds(&mut surface, "archive-tree-icon-folder");
    let folder_arrow = bounds(&mut surface, "archive-tree-disclosure-folder");
    assert_eq!(root_arrow.size, size(px(16.), px(16.)));
    assert_eq!(root.left() - root_arrow.right(), px(4.));
    assert_eq!(folder.left() - root.left(), px(16.));
    assert_eq!(folder_arrow.left() - root_arrow.left(), px(16.));
    pointer_click(&mut surface, folder_arrow);
    let child = bounds(&mut surface, "archive-tree-icon-folder/deep");
    assert_eq!(child.left() - folder.left(), px(16.));
    let root_arrow = bounds(&mut surface, "archive-tree-disclosure-");
    pointer_click(&mut surface, root_arrow);
    assert!(
        surface.debug_bounds("archive-tree-icon-folder").is_none(),
        "official root collapse owns children"
    );
    let root_arrow = bounds(&mut surface, "archive-tree-disclosure-");
    pointer_click(&mut surface, root_arrow);
    assert!(surface.debug_bounds("archive-tree-icon-folder").is_some());
    assert!(owner.read_with(&surface, |app, _| app.windows.contains_key(&ids[0])));
    close(&owner, &mut surface);
}
